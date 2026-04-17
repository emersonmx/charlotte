use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::{
    HeaderMap, Method, Uri, body::Bytes, header::HeaderValue, service::service_fn,
    upgrade::Upgraded,
};
use hyper_util::rt::TokioIo;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
};

type ClientBuilder = hyper::client::conn::http1::Builder;
type ServerBuilder = hyper::server::conn::http1::Builder;
type IncomingRequest = hyper::Request<hyper::body::Incoming>;
type IncomingResponse = hyper::Response<hyper::body::Incoming>;
type HyperRequest = hyper::Request<BoxBody<Bytes, Error>>;
type HyperResponse = hyper::Response<BoxBody<Bytes, Error>>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("http error: {0}")]
    Http(#[from] hyper::http::Error),
    #[error("invalid uri: {0}")]
    InvalidUri(#[from] hyper::http::uri::InvalidUri),
    #[error("{0}")]
    Proxy(String),
}

pub type RequestId = usize;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    async fn from_parts(
        parts: &hyper::http::request::Parts,
        body: BoxBody<Bytes, Error>,
    ) -> Result<Self, Error> {
        let body = body.collect().await?.to_bytes().to_vec();

        Ok(Self {
            method: parts.method.to_string(),
            url: parts.uri.to_string(),
            headers: headers_to_vec(&parts.headers),
            body,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    async fn from_parts(
        parts: &hyper::http::response::Parts,
        body: BoxBody<Bytes, Error>,
    ) -> Result<Self, Error> {
        let body = body.collect().await?.to_bytes().to_vec();

        Ok(Self {
            status: parts.status.as_u16(),
            headers: headers_to_vec(&parts.headers),
            body,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Message {
    ServerStarted,
    ServerStopped,
    ClientConnected(std::net::SocketAddr),
    ClientDisconnected(std::net::SocketAddr),
    RequestSent((RequestId, Request)),
    ResponseReceived((RequestId, Response)),
    ErrorOccurred((Option<RequestId>, Error)),
}

pub struct Server {
    addr: std::net::SocketAddr,
    message_channel: mpsc::Sender<Message>,
    request_id_counter: Arc<AtomicUsize>,
}

impl Server {
    pub fn new(addr: std::net::SocketAddr, message_channel: mpsc::Sender<Message>) -> Arc<Self> {
        let request_id_counter = Arc::new(AtomicUsize::new(1));
        Arc::new(Self {
            addr,
            message_channel,
            request_id_counter,
        })
    }

    pub async fn run(
        self: Arc<Self>,
        mut abort_channel: oneshot::Receiver<()>,
    ) -> Result<(), Error> {
        let listener = TcpListener::bind(self.addr).await?;

        let _ = self.message_channel.send(Message::ServerStarted).await;

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    let (socket, socket_addr) = accept_result?;

                    let _ = self.message_channel
                        .send(Message::ClientConnected(socket_addr))
                        .await;

                    let handler = {
                        let self_clone = self.clone();
                        move |req| {
                            let self_clone = self_clone.clone();
                            self_clone.proxy_handle(req, socket_addr)
                        }
                    };

                    let message_channel = self.message_channel.clone();
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

        let _ = self.message_channel.send(Message::ServerStopped).await;

        Ok(())
    }

    async fn proxy_handle(
        self: Arc<Self>,
        req: IncomingRequest,
        client_addr: std::net::SocketAddr,
    ) -> Result<HyperResponse, Error> {
        match req.method() {
            &Method::CONNECT => self.handle_connect(req, client_addr).await,
            _ => self.handle_regular(req, client_addr, false).await,
        }
    }

    async fn handle_connect(
        self: Arc<Self>,
        req: IncomingRequest,
        client_addr: std::net::SocketAddr,
    ) -> Result<HyperResponse, Error> {
        let request_id = self.request_id_counter.fetch_add(1, Ordering::Relaxed);

        let (parts, body) = extract_request_parts(req).await?;
        let req_body = boxed_body_from_bytes(body.clone());
        let channel_body = boxed_body_from_bytes(body);

        let request = Request::from_parts(&parts, channel_body).await?;
        let _ = self
            .message_channel
            .send(Message::RequestSent((request_id, request)))
            .await;

        let req = HyperRequest::from_parts(parts, req_body);

        if let Ok(addr) = get_target_addr(req.uri(), req.headers()) {
            let self_clone = self.clone();
            let message_channel = self_clone.message_channel.clone();
            tokio::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        if let Err(e) = self_clone
                            .handle_upgraded(upgraded, &addr, client_addr)
                            .await
                        {
                            let _ = message_channel
                                .send(Message::ErrorOccurred((Some(request_id), e)))
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
        let mut res = HyperResponse::new(empty);
        *res.status_mut() = hyper::StatusCode::OK;

        let (parts, body) = extract_response_parts(res).await?;
        let res_body = boxed_body_from_bytes(body.clone());
        let channel_body = boxed_body_from_bytes(body);
        let response = Response::from_parts(&parts, channel_body).await?;
        let _ = self
            .message_channel
            .send(Message::ResponseReceived((request_id, response)))
            .await;

        let res = HyperResponse::from_parts(parts, res_body);
        Ok(res)
    }

    async fn handle_upgraded(
        self: Arc<Self>,
        upgraded: Upgraded,
        addr: &str,
        client_addr: std::net::SocketAddr,
    ) -> Result<(), Error> {
        let request_id = self.request_id_counter.load(Ordering::Relaxed);

        let ca_cert_bytes = std::fs::read("certs/ca.crt")?;
        let ca_key_bytes = std::fs::read("certs/ca.key")?;
        let ca_cert_der = rustls::pki_types::CertificateDer::from(ca_cert_bytes);
        let ca_key_der = rustls::pki_types::PrivatePkcs8KeyDer::from(ca_key_bytes);

        let ca_key_pair = rcgen::KeyPair::try_from(&ca_key_der)
            .map_err(|e| Error::Proxy(format!("Failed to parse CA key: {e}")))?;
        let issuer = rcgen::Issuer::from_ca_cert_der(&ca_cert_der, ca_key_pair)
            .map_err(|e| Error::Proxy(format!("Failed to create issuer from CA cert: {e}")))?;

        let domain = addr.split(':').next().unwrap_or("localhost");
        let mut params = rcgen::CertificateParams::new(vec![domain.to_string()])
            .map_err(|e| Error::Proxy(format!("Failed to create certificate params: {e}")))?;
        params.is_ca = rcgen::IsCa::NoCa;
        let domain_keypair = rcgen::KeyPair::generate()
            .map_err(|e| Error::Proxy(format!("Failed to generate key pair: {e}")))?;
        let domain_cert = params
            .signed_by(&domain_keypair, &issuer)
            .map_err(|e| Error::Proxy(format!("Failed to sign certificate: {e}")))?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![domain_cert.into()], domain_keypair.into())
            .map_err(|e| Error::Proxy(format!("Failed to create TLS config: {e}")))?;

        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));

        let tls_stream = acceptor.accept(TokioIo::new(upgraded)).await?;

        let handler = {
            let self_clone = self.clone();
            move |req| {
                let self_clone = self_clone.clone();
                self_clone.handle_regular(req, client_addr, true)
            }
        };

        tokio::spawn(async move {
            if let Err(err) = ServerBuilder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(TokioIo::new(tls_stream), service_fn(handler))
                .with_upgrades()
                .await
            {
                let _ = self
                    .message_channel
                    .send(Message::ErrorOccurred((Some(request_id), err.into())))
                    .await;
            }

            let _ = self
                .message_channel
                .send(Message::ClientDisconnected(client_addr))
                .await;
        });

        Ok(())
    }

    async fn handle_regular(
        self: Arc<Self>,
        req: IncomingRequest,
        _client_addr: std::net::SocketAddr,
        is_tls: bool,
    ) -> Result<HyperResponse, Error> {
        let request_id = self.request_id_counter.fetch_add(1, Ordering::Relaxed);

        let (mut parts, body) = extract_request_parts(req).await?;
        fix_relative_uri(&mut parts, is_tls)?;
        let fetch_body = boxed_body_from_bytes(body.clone());
        let channel_body = boxed_body_from_bytes(body);

        let request = Request::from_parts(&parts, channel_body).await?;
        let _ = self
            .message_channel
            .send(Message::RequestSent((request_id, request)))
            .await;

        let res = fetch(HyperRequest::from_parts(parts, fetch_body)).await?;

        let (parts, body) = extract_response_parts(res).await?;
        let client_body = boxed_body_from_bytes(body.clone());
        let channel_body = boxed_body_from_bytes(body);

        let response = Response::from_parts(&parts, channel_body).await?;
        let _ = self
            .message_channel
            .send(Message::ResponseReceived((request_id, response)))
            .await;

        Ok(HyperResponse::from_parts(parts, client_body))
    }
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

async fn extract_request_parts<B>(
    req: hyper::Request<B>,
) -> Result<(hyper::http::request::Parts, Bytes), Error>
where
    B: hyper::body::Body,
    B::Error: std::fmt::Debug,
{
    let (mut parts, body) = req.into_parts();
    clean_request_headers(&mut parts.headers);
    let body = body
        .collect()
        .await
        .map_err(|e| Error::Proxy(format!("Body error: {e:?}")))?
        .to_bytes();
    Ok((parts, body))
}

fn clean_request_headers(headers: &mut HeaderMap) {
    headers.remove(hyper::header::CONNECTION);
    headers.remove("proxy-connection");
    headers.remove("keep-alive");
    headers.remove(hyper::header::TRANSFER_ENCODING);
}

async fn extract_response_parts<B>(
    res: hyper::Response<B>,
) -> Result<(hyper::http::response::Parts, Bytes), Error>
where
    B: hyper::body::Body,
    B::Error: std::fmt::Debug,
{
    let (mut parts, body) = res.into_parts();
    clean_response_headers(&mut parts.headers);
    let body = body
        .collect()
        .await
        .map_err(|e| Error::Proxy(format!("Body error: {e:?}")))?
        .to_bytes();
    Ok((parts, body))
}

fn clean_response_headers(headers: &mut HeaderMap) {
    headers.remove(hyper::header::CONNECTION);
    headers.remove(hyper::header::TRANSFER_ENCODING);
}

fn boxed_body_from_bytes(body: Bytes) -> BoxBody<Bytes, Error> {
    Full::new(body).map_err(|e| match e {}).boxed()
}

fn headers_to_vec(headers: &HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                v.to_str()
                    .unwrap_or("<invalid UTF-8 in header value>")
                    .to_string(),
            )
        })
        .collect()
}

async fn fetch(req: HyperRequest) -> Result<IncomingResponse, Error> {
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

    let req = HyperRequest::from_parts(parts, body);
    let res = sender.send_request(req).await?;

    Ok(res)
}

fn fix_relative_uri(parts: &mut hyper::http::request::Parts, is_tls: bool) -> Result<(), Error> {
    if parts.uri.scheme().is_none()
        && let Some(host) = parts.headers.get(hyper::header::HOST)
    {
        let host = host
            .to_str()
            .map_err(|_| Error::Proxy("Host inválido".to_string()))?;
        let scheme = if is_tls { "https" } else { "http" };
        let new_uri = format!("{}://{}{}", scheme, host, parts.uri);
        let uri = new_uri.parse::<Uri>()?;
        parts.uri = uri;
    }
    Ok(())
}
