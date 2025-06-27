use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::{Request, Response, body::Bytes, header::HeaderValue, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};

type ClientBuilder = hyper::client::conn::http1::Builder;
type ServerBuilder = hyper::server::conn::http1::Builder;

#[derive(thiserror::Error, Debug)]
enum Error {
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_addr = "127.0.0.1:8888";
    let listener = TcpListener::bind(server_addr).await?;

    println!("Listening on http://{}", server_addr);
    loop {
        let (socket, socket_addr) = listener.accept().await?;
        println!("Client '{}' connected", socket_addr);

        let io = TokioIo::new(socket);
        tokio::spawn(async move {
            if let Err(err) = ServerBuilder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, service_fn(proxy))
                .with_upgrades()
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn proxy(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, Error>>, Error> {
    let prepared_req = make_request(req).await?;

    let backend_res = fetch(prepared_req).await?;

    let client_res = make_response(backend_res).await?;
    Ok(client_res)
}

async fn make_request(
    req: Request<hyper::body::Incoming>,
) -> Result<Request<BoxBody<Bytes, Error>>, Error> {
    let (parts, incoming_body) = req.into_parts();
    let body = incoming_body
        .collect()
        .await?
        .map_err(|never| match never {})
        .boxed();

    let req = Request::from_parts(parts, body);
    Ok(req)
}

async fn make_response(
    res: Response<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, Error>>, Error> {
    let (parts, incoming_body) = res.into_parts();
    let body = incoming_body
        .collect()
        .await?
        .map_err(|never| match never {})
        .boxed();

    let res = Response::from_parts(parts, body);
    Ok(res)
}

async fn fetch(
    req: Request<BoxBody<Bytes, Error>>,
) -> Result<Response<hyper::body::Incoming>, Error> {
    let (mut parts, body) = req.into_parts();

    let addr = get_target_addr(&parts)?;
    let stream = TcpStream::connect(addr.clone()).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = ClientBuilder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .handshake(io)
        .await?;
    tokio::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("Connection failed: {:?}", err);
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

fn get_target_addr(parts: &hyper::http::request::Parts) -> Result<String, Error> {
    if let Some(host) = parts.uri.host() {
        let port = parts.uri.port_u16().unwrap_or(80);
        Ok(format!("{}:{}", host, port))
    } else if let Some(host) = parts.headers.get(hyper::header::HOST) {
        let default_port = match parts.uri.scheme_str() {
            Some("https") => 443_u16,
            _ => 80_u16,
        };

        let host_str = host
            .to_str()
            .map_err(|_| Error::Proxy("Invalid HOST header format (not UTF-8)".to_string()))?;

        match host_str.rsplit_once(':') {
            Some((host, port_str)) => {
                let port = port_str.parse::<u16>().unwrap_or(default_port);
                Ok(format!("{}:{}", host, port))
            }
            None => Ok(format!("{}:{}", host_str, default_port)),
        }
    } else {
        Err(Error::Proxy(
            "Missing host information in URI or Host header".to_string(),
        ))
    }
}
