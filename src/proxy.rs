use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::{
    HeaderMap, Method, Uri, body::Bytes, header::HeaderValue, service::service_fn,
    upgrade::Upgraded,
};
use hyper_util::rt::TokioIo;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
};

type ClientBuilder = hyper::client::conn::http1::Builder;
type IncomingRequest = hyper::Request<hyper::body::Incoming>;
type IncomingResponse = hyper::Response<hyper::body::Incoming>;
type ServerBuilder = hyper::server::conn::http1::Builder;

pub type RequestId = u32;
pub type Request = hyper::Request<BoxBody<Bytes, Error>>;
pub type Response = hyper::Response<BoxBody<Bytes, Error>>;

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

#[derive(Debug)]
pub enum Message {
    ServerStarted(std::net::SocketAddr),
    ServerStopped(std::net::SocketAddr),
    ClientConnected(std::net::SocketAddr),
    ClientDisconnected(std::net::SocketAddr),
    RequestSent((RequestId, Request)),
    ResponseReceived((RequestId, Response)),
    ErrorOccurred((Option<RequestId>, Error)),
}

pub async fn serve(
    server_addr: std::net::SocketAddr,
    message_channel: mpsc::Sender<Message>,
    mut abort_channel: oneshot::Receiver<()>,
) -> Result<(), Error> {
    let listener = TcpListener::bind(server_addr).await?;

    let _ = message_channel
        .send(Message::ServerStarted(server_addr))
        .await;

    let request_id_counter = Arc::new(AtomicU32::new(1));
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let (socket, socket_addr) = accept_result?;

                let message_channel = message_channel.clone();
                let _ = message_channel
                    .send(Message::ClientConnected(socket_addr))
                    .await;

                let handler = {
                    let message_channel = message_channel.clone();
                    let request_id_counter = request_id_counter.clone();
                    move |req| {
                        let request_id = request_id_counter.fetch_add(1, Ordering::Relaxed);
                        proxy_handle(req, message_channel.clone(), request_id)
                    }
                };

                tokio::spawn(async move {
                    if let Err(err) = ServerBuilder::new()
                        .preserve_header_case(true)
                            .title_case_headers(true)
                            .serve_connection(
                                TokioIo::new(socket),
                                service_fn(handler),
                            )
                            .with_upgrades()
                            .await
                    {
                        let _ = message_channel
                            .send(Message::ErrorOccurred((None, err.into())))
                            .await;
                    }

                    let _ = message_channel
                        .send(Message::ClientDisconnected(socket_addr))
                        .await;
                    });
            }
            _ = &mut abort_channel => break
        }
    }

    let _ = message_channel
        .send(Message::ServerStopped(server_addr))
        .await;

    Ok(())
}

async fn proxy_handle(
    req: IncomingRequest,
    message_channel: mpsc::Sender<Message>,
    request_id: RequestId,
) -> Result<Response, Error> {
    match req.method() {
        &Method::CONNECT => handle_connect(req, message_channel, request_id).await,
        _ => handle_regular(req, message_channel, request_id).await,
    }
}

async fn handle_connect(
    req: IncomingRequest,
    message_channel: mpsc::Sender<Message>,
    request_id: RequestId,
) -> Result<Response, Error> {
    if let Ok(addr) = get_target_addr(req.uri(), req.headers()) {
        tokio::spawn(async move {
            match hyper::upgrade::on(req).await {
                Ok(upgraded) => {
                    if let Err(e) = tunnel(upgraded, &addr).await {
                        let _ = message_channel
                            .send(Message::ErrorOccurred((Some(request_id), e.into())))
                            .await;
                    }
                }
                Err(e) => {
                    let _ = message_channel
                        .send(Message::ErrorOccurred((Some(request_id), e.into())))
                        .await;
                }
            }
        });
    }

    let empty = Empty::<Bytes>::new().map_err(|e| match e {}).boxed();
    let mut res = Response::new(empty);
    *res.status_mut() = hyper::StatusCode::OK;
    Ok(res)
}

async fn handle_regular(
    req: IncomingRequest,
    message_channel: mpsc::Sender<Message>,
    request_id: RequestId,
) -> Result<Response, Error> {
    let (parts, body) = extract_request_parts(req).await?;
    let fetch_body = boxed_body_from_bytes(body.clone());
    let channel_body = boxed_body_from_bytes(body);

    let _ = message_channel
        .send(Message::RequestSent((
            request_id,
            Request::from_parts(parts.clone(), channel_body),
        )))
        .await;

    let res = fetch(Request::from_parts(parts, fetch_body)).await?;

    let (parts, body) = extract_response_parts(res).await?;
    let client_body = boxed_body_from_bytes(body.clone());
    let channel_body = boxed_body_from_bytes(body);

    let _ = message_channel
        .send(Message::ResponseReceived((
            request_id,
            Response::from_parts(parts.clone(), channel_body),
        )))
        .await;

    Ok(Response::from_parts(parts, client_body))
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

async fn tunnel(upgraded: Upgraded, addr: &str) -> std::io::Result<()> {
    let mut server = TcpStream::connect(addr).await?;
    let mut upgraded = TokioIo::new(upgraded);

    let (from_client, from_server) =
        tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;

    println!("Client wrote {from_client} bytes and received {from_server} bytes");

    Ok(())
}

async fn extract_request_parts(
    req: IncomingRequest,
) -> Result<(hyper::http::request::Parts, Bytes), Error> {
    let (mut parts, body) = req.into_parts();
    clean_request_headers(&mut parts.headers);
    let body = body.collect().await?.to_bytes();
    Ok((parts, body))
}

fn clean_request_headers(headers: &mut HeaderMap) {
    headers.remove(hyper::header::CONNECTION);
    headers.remove("proxy-connection");
    headers.remove("keep-alive");
    headers.remove(hyper::header::TRANSFER_ENCODING);
}

fn boxed_body_from_bytes(body: Bytes) -> BoxBody<Bytes, Error> {
    Full::new(body).map_err(|e| match e {}).boxed()
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

async fn extract_response_parts(
    res: IncomingResponse,
) -> Result<(hyper::http::response::Parts, Bytes), Error> {
    let (mut parts, body) = res.into_parts();
    clean_response_headers(&mut parts.headers);
    let body = body.collect().await?.to_bytes();
    Ok((parts, body))
}

fn clean_response_headers(headers: &mut HeaderMap) {
    headers.remove(hyper::header::CONNECTION);
    headers.remove(hyper::header::TRANSFER_ENCODING);
}
