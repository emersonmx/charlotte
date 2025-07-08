use http_body_util::{BodyExt, Empty, combinators::BoxBody};
use hyper::{HeaderMap, Method, Uri, body::Bytes, header::HeaderValue, upgrade::Upgraded};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

type ClientBuilder = hyper::client::conn::http1::Builder;
type IncomingRequest = hyper::Request<hyper::body::Incoming>;
type IncomingResponse = hyper::Response<hyper::body::Incoming>;
type Request = hyper::Request<BoxBody<Bytes, Error>>;
type Response = hyper::Response<BoxBody<Bytes, Error>>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("hyper error")]
    Hyper(#[from] hyper::Error),
    #[error("http error")]
    Http(#[from] hyper::http::Error),
    #[error("invalid uri")]
    InvalidUri(#[from] hyper::http::uri::InvalidUri),
    #[error("{0}")]
    Proxy(String),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Proxy;

impl Proxy {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn handle(&self, req: IncomingRequest) -> Result<Response, Error> {
        if req.method() == Method::CONNECT {
            if let Ok(addr) = get_target_addr(req.uri(), req.headers()) {
                tokio::spawn(async move {
                    match hyper::upgrade::on(req).await {
                        Ok(upgraded) => {
                            if let Err(e) = tunnel(upgraded, &addr).await {
                                eprintln!("Server io error: {e}");
                            }
                        }
                        Err(e) => eprintln!("Upgrade error: {e}"),
                    }
                });
            }

            let empty = Empty::<Bytes>::new().map_err(|e| match e {}).boxed();
            let mut res = Response::new(empty);
            *res.status_mut() = hyper::StatusCode::OK;
            Ok(res)
        } else {
            let prepared_req = make_request(req).await?;

            let backend_res = fetch(prepared_req).await?;

            let client_res = make_response(backend_res).await?;
            Ok(client_res)
        }
    }
}

async fn make_request(req: IncomingRequest) -> Result<Request, Error> {
    let (mut parts, incoming_body) = req.into_parts();
    let body = incoming_body
        .collect()
        .await?
        .map_err(|never| match never {})
        .boxed();

    parts.headers.remove(hyper::header::CONNECTION);
    parts.headers.remove("proxy-connection");
    parts.headers.remove("keep-alive");
    parts.headers.remove(hyper::header::TRANSFER_ENCODING);

    let req = Request::from_parts(parts, body);
    Ok(req)
}

async fn make_response(res: IncomingResponse) -> Result<Response, Error> {
    let (mut parts, incoming_body) = res.into_parts();
    let body = incoming_body
        .collect()
        .await?
        .map_err(|never| match never {})
        .boxed();

    parts.headers.remove(hyper::header::CONNECTION);
    parts.headers.remove(hyper::header::TRANSFER_ENCODING);

    let res = Response::from_parts(parts, body);
    Ok(res)
}

async fn fetch(req: Request) -> Result<IncomingResponse, Error> {
    let (mut parts, body) = req.into_parts();

    let addr = get_target_addr(&parts.uri, &parts.headers)?;
    let stream = TcpStream::connect(addr.clone()).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = ClientBuilder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .handshake(io)
        .await?;
    tokio::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("Connection failed: {err:?}");
        }
    });

    while parts.headers.get(hyper::header::HOST).is_some() {
        parts.headers.remove(hyper::header::HOST);
    }
    let host_value = HeaderValue::from_str(&addr)
        .map_err(|_| Error::Proxy("Could not parse the host header".to_string()))?;
    let _ = parts.headers.insert(hyper::header::HOST, host_value);

    let req = Request::from_parts(parts, body);

    let res = sender.send_request(req).await?;
    Ok(res)
}

async fn tunnel(upgraded: Upgraded, addr: &str) -> std::io::Result<()> {
    let mut server = TcpStream::connect(addr).await?;
    let mut upgraded = TokioIo::new(upgraded);

    let (from_client, from_server) =
        tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;

    println!("Client wrote {from_client} bytes and received {from_server} bytes");

    Ok(())
}

fn get_target_addr(uri: &Uri, headers: &HeaderMap) -> Result<String, Error> {
    let host = get_host(uri, headers)?;
    let port = get_port(uri, headers)?;
    Ok(format!("{host}:{port}"))
}

fn get_host(uri: &Uri, headers: &HeaderMap) -> Result<String, Error> {
    if let Some(host) = uri.host() {
        Ok(host.to_string())
    } else if let Some(host) = headers.get(hyper::header::HOST) {
        let host_str = host
            .to_str()
            .map_err(|_| Error::Proxy("Invalid HOST header format (not UTF-8)".to_string()))?;

        match host_str.rsplit_once(':') {
            Some((host, _)) => Ok(host.to_string()),
            None => Ok(host_str.to_string()),
        }
    } else {
        Err(Error::Proxy(
            "Missing host information in URI or Host header".to_string(),
        ))
    }
}

fn get_port(uri: &Uri, headers: &HeaderMap) -> Result<String, Error> {
    let default_port = match uri.scheme_str() {
        Some("https") => 443_u16,
        _ => 80_u16,
    };

    if uri.host().is_some() {
        let port = uri.port_u16().unwrap_or(default_port);
        Ok(port.to_string())
    } else if let Some(host) = headers.get(hyper::header::HOST) {
        let host_str = host
            .to_str()
            .map_err(|_| Error::Proxy("Invalid HOST header format (not UTF-8)".to_string()))?;

        match host_str.rsplit_once(':') {
            Some((_, port_str)) => {
                let port = port_str.parse::<u16>().unwrap_or(default_port);
                Ok(port.to_string())
            }
            None => Ok(default_port.to_string()),
        }
    } else {
        Err(Error::Proxy(
            "Missing port information in URI or Host header".to_string(),
        ))
    }
}
