use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::{
    HeaderMap, Method, Uri, body::Bytes, header::HeaderValue, service::service_fn,
    upgrade::Upgraded,
};
use hyper_util::rt::TokioIo;
use rustls::pki_types::{DnsName, ServerName};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
};
use tokio_rustls::{
    TlsConnector,
    rustls::{ClientConfig, RootCertStore},
};

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type ClientBuilder = hyper::client::conn::http1::Builder;
type ServerBuilder = hyper::server::conn::http1::Builder;
type IncomingRequest = hyper::Request<hyper::body::Incoming>;
type IncomingResponse = hyper::Response<hyper::body::Incoming>;
type HyperRequest = hyper::Request<BoxBody<Bytes, Error>>;
type HyperResponse = hyper::Response<BoxBody<Bytes, Error>>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to listen on address: {addr}, error: {source}")]
    BindAddress {
        addr: std::net::SocketAddr,
        #[source]
        source: BoxError,
    },
    #[error("Failed to accept connection: {0}")]
    AcceptConnection(#[source] BoxError),
    #[error("{message} (uri: {uri}, headers: {headers:?})")]
    TargetAddress {
        message: String,
        uri: String,
        headers: Vec<(String, String)>,
    },
    #[error("Failed to fix relative URI: {0:?}")]
    FixRelativeUri(Box<RequestContextError>),

    #[error("Failed to establish connection to target server: {message}, error: {source}")]
    TargetConnection {
        message: String,
        #[source]
        source: BoxError,
    },
    #[error("Failed to fetch from target server: {message}, error: {source}")]
    Fetch {
        message: String,
        #[source]
        source: BoxError,
    },

    #[error("Failed to read request body: {0:?}")]
    RequestBodyRead(Box<RequestContextError>),
    #[error("Failed to read response body: {0:?}")]
    ResponseBodyRead(Box<ResponseContextError>),

    #[error("Failed to send message: {message:?}, error: {source}")]
    Channel {
        message: Box<Message>,
        #[source]
        source: BoxError,
    },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequestContextError {
    pub method: String,
    pub uri: String,
    pub version: String,
    pub headers: Vec<(String, String)>,
    pub extensions: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResponseContextError {
    pub status: u16,
    pub version: String,
    pub headers: Vec<(String, String)>,
    pub extensions: String,
    pub message: String,
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
    ErrorOccurred((Option<RequestId>, Box<Error>)),
}

pub struct CertificateStore {
    ca_cert_bytes: Vec<u8>,
    ca_key_bytes: Vec<u8>,
}

impl CertificateStore {
    pub fn new(ca_cert_bytes: &[u8], ca_key_bytes: &[u8]) -> Result<Self, BoxError> {
        Ok(Self {
            ca_cert_bytes: ca_cert_bytes.to_vec(),
            ca_key_bytes: ca_key_bytes.to_vec(),
        })
    }

    pub fn from_files(ca_cert_path: &Path, ca_key_path: &Path) -> Result<Self, BoxError> {
        let ca_cert_bytes = std::fs::read(ca_cert_path).map_err(|e| {
            format!(
                "Failed to read CA certificate from {:?}: {e}",
                &ca_cert_path
            )
        })?;
        let ca_key_bytes = std::fs::read(ca_key_path)
            .map_err(|e| format!("Failed to read CA key from {:?}: {e}", &ca_key_path))?;
        Self::new(&ca_cert_bytes, &ca_key_bytes)
    }

    pub fn generate_cert(
        &self,
        domain: &str,
    ) -> Result<(rcgen::Certificate, rcgen::KeyPair), BoxError> {
        let issuer = self.get_issuer()?;

        let mut params = rcgen::CertificateParams::new(vec![domain.to_string()])
            .map_err(|e| format!("Failed to create certificate params: {e}"))?;
        params.is_ca = rcgen::IsCa::NoCa;
        let domain_keypair =
            rcgen::KeyPair::generate().map_err(|e| format!("Failed to generate key pair: {e}"))?;
        let domain_cert = params
            .signed_by(&domain_keypair, &issuer)
            .map_err(|e| format!("Failed to sign certificate: {e}"))?;

        Ok((domain_cert, domain_keypair))
    }

    fn get_issuer(&self) -> Result<rcgen::Issuer<'_, rcgen::KeyPair>, BoxError> {
        let ca_cert_der = rustls::pki_types::CertificateDer::from(self.ca_cert_bytes.clone());
        let ca_key_der = rustls::pki_types::PrivatePkcs8KeyDer::from(self.ca_key_bytes.clone());

        let ca_key_pair = rcgen::KeyPair::try_from(&ca_key_der)
            .map_err(|e| format!("Failed to parse CA key: {e}"))?;
        let issuer = rcgen::Issuer::from_ca_cert_der(&ca_cert_der, ca_key_pair)
            .map_err(|e| format!("Failed to create issuer from CA cert: {e}"))?;

        Ok(issuer)
    }
}

pub struct Server {
    addr: std::net::SocketAddr,
    message_channel: mpsc::Sender<Message>,
    request_id_counter: Arc<AtomicUsize>,
    certificate_store: CertificateStore,
    root_cert_store: RootCertStore,
}

impl Server {
    pub fn new(
        addr: std::net::SocketAddr,
        certificate_store: CertificateStore,
        message_channel: mpsc::Sender<Message>,
    ) -> Self {
        let request_id_counter = Arc::new(AtomicUsize::new(1));
        let root_cert_store =
            RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        Self {
            addr,
            message_channel,
            request_id_counter,
            certificate_store,
            root_cert_store,
        }
    }

    pub async fn run(self, mut abort_channel: oneshot::Receiver<()>) -> Result<(), Error> {
        let self_arc = Arc::new(self);
        let listener = TcpListener::bind(self_arc.addr)
            .await
            .map_err(|e| Error::BindAddress {
                addr: self_arc.addr,
                source: e.into(),
            })?;

        self_arc
            .message_channel
            .send(Message::ServerStarted)
            .await
            .map_err(|e| Error::Channel {
                message: Message::ServerStarted.into(),
                source: e.into(),
            })?;

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    let (socket, socket_addr) = accept_result
                        .map_err(|e| Error::AcceptConnection(e.into()))?;

                    self_arc.message_channel
                        .send(Message::ClientConnected(socket_addr))
                        .await.map_err(|e| Error::Channel {
                            message: Message::ClientConnected(socket_addr).into(),
                            source: e.into(),
                        })?;

                    let handler = {
                        let self_clone = self_arc.clone();
                        move |req| {
                            let self_clone = self_clone.clone();
                            self_clone.proxy_handle(req, socket_addr)
                        }
                    };

                    let message_channel = self_arc.message_channel.clone();
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
                                .send(Message::ErrorOccurred((
                                    None,
                                    Error::TargetConnection {
                                        message: "Error handling client connection".to_string(),
                                        source: err.into()
                                    }
                                    .into(),
                                )))
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

        self_arc
            .message_channel
            .send(Message::ServerStopped)
            .await
            .map_err(|e| Error::Channel {
                message: Message::ServerStopped.into(),
                source: e.into(),
            })?;

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
                                .send(Message::ErrorOccurred((Some(request_id), e.into())))
                                .await;
                        }
                    }
                    Err(e) => {
                        let _ = message_channel
                            .send(Message::ErrorOccurred((
                                Some(request_id),
                                Error::TargetConnection {
                                    message: "Failed to upgrade connection for CONNECT request"
                                        .to_string(),
                                    source: e.into(),
                                }
                                .into(),
                            )))
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

        let domain = addr.split(':').next().unwrap_or("localhost");
        let (domain_cert, domain_keypair) =
            self.certificate_store
                .generate_cert(domain)
                .map_err(|e| Error::TargetConnection {
                    message: format!("Failed to generate certificate for {domain}: {e}"),
                    source: e,
                })?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![domain_cert.into()], domain_keypair.into())
            .map_err(|e| Error::TargetConnection {
                message: format!("Failed to create TLS config: {e}"),
                source: e.into(),
            })?;

        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));

        let tls_stream =
            acceptor
                .accept(TokioIo::new(upgraded))
                .await
                .map_err(|e| Error::TargetConnection {
                    message: "Failed to establish TLS connection with client".to_string(),
                    source: e.into(),
                })?;

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
                    .send(Message::ErrorOccurred((
                        Some(request_id),
                        Error::TargetConnection {
                            message: "Error handling upgraded TLS connection".to_string(),
                            source: err.into(),
                        }
                        .into(),
                    )))
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

        let self_clone = self.clone();
        let res = self_clone
            .fetch(HyperRequest::from_parts(parts, fetch_body))
            .await?;

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

    async fn fetch(self: Arc<Self>, req: HyperRequest) -> Result<IncomingResponse, Error> {
        let (mut parts, body) = req.into_parts();

        let is_tls = parts.uri.scheme_str() == Some("https");
        let host = get_host(&parts.uri, &parts.headers)?;
        let addr = get_target_addr(&parts.uri, &parts.headers)?;

        let stream = TcpStream::connect(addr.clone())
            .await
            .map_err(|e| Error::Fetch {
                message: format!("Failed to connect to target server at {addr}"),
                source: e.into(),
            })?;

        while parts.headers.get(hyper::header::HOST).is_some() {
            parts.headers.remove(hyper::header::HOST);
        }
        let host_value = HeaderValue::from_str(&addr).map_err(|e| Error::Fetch {
            message: "Could not parse the host header".to_string(),
            source: e.into(),
        })?;
        let _ = parts.headers.insert(hyper::header::HOST, host_value);

        let req = HyperRequest::from_parts(parts, body);

        if is_tls {
            self.fetch_tls(req, stream, host, addr).await
        } else {
            self.fetch_plain(req, stream).await
        }
    }

    async fn fetch_tls(
        self: Arc<Self>,
        req: HyperRequest,
        stream: TcpStream,
        host: String,
        addr: String,
    ) -> Result<IncomingResponse, Error> {
        let config = ClientConfig::builder()
            .with_root_certificates(self.root_cert_store.clone())
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(config));
        let dnsname = DnsName::try_from_str(&host).map_err(|e| Error::Fetch {
            message: format!("Invalid DNS name for TLS connection: {host}"),
            source: e.into(),
        })?;

        let stream = connector
            .connect(ServerName::DnsName(dnsname).to_owned(), stream)
            .await
            .map_err(|e| Error::Fetch {
                message: format!("Failed to establish TLS connection to target server at {addr}"),
                source: e.into(),
            })?;

        let io = TokioIo::new(stream);

        let (mut sender, conn) = ClientBuilder::new()
            .preserve_header_case(true)
            .title_case_headers(true)
            .handshake(io)
            .await
            .map_err(|e| Error::Fetch {
                message: "Failed to handshake with TLS target server".to_string(),
                source: e.into(),
            })?;

        tokio::spawn(async move {
            if let Err(err) = conn.await {
                let _ = self
                    .message_channel
                    .send(Message::ErrorOccurred((
                        None,
                        Error::Fetch {
                            message: "Connection error with TLS target server".to_string(),
                            source: err.into(),
                        }
                        .into(),
                    )))
                    .await;
            }
        });

        sender.send_request(req).await.map_err(|e| Error::Fetch {
            message: "Failed to fetch response from TLS target server".to_string(),
            source: e.into(),
        })
    }

    async fn fetch_plain(
        self: Arc<Self>,
        req: HyperRequest,
        stream: TcpStream,
    ) -> Result<IncomingResponse, Error> {
        let io = TokioIo::new(stream);

        let (mut sender, conn) = ClientBuilder::new()
            .preserve_header_case(true)
            .title_case_headers(true)
            .handshake(io)
            .await
            .map_err(|e| Error::Fetch {
                message: "Failed to establish connection to target server".to_string(),
                source: e.into(),
            })?;

        tokio::spawn(async move {
            if let Err(err) = conn.await {
                let _ = self
                    .message_channel
                    .send(Message::ErrorOccurred((
                        None,
                        Error::Fetch {
                            message: "Connection error with target server".to_string(),
                            source: err.into(),
                        }
                        .into(),
                    )))
                    .await;
            }
        });

        sender.send_request(req).await.map_err(|e| Error::Fetch {
            message: "Failed to fetch response from target server".to_string(),
            source: e.into(),
        })
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
        let host_str = host.to_str().map_err(|_| Error::TargetAddress {
            message: "Invalid HOST header format (not UTF-8)".to_string(),
            uri: uri.to_string(),
            headers: headers_to_vec(headers),
        })?;

        match host_str.rsplit_once(':') {
            Some((host, _)) => Ok(host.to_string()),
            None => Ok(host_str.to_string()),
        }
    } else {
        Err(Error::TargetAddress {
            message: "Missing host information in URI or Host header".to_string(),
            uri: uri.to_string(),
            headers: headers_to_vec(headers),
        })
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
        let host_str = host.to_str().map_err(|_| Error::TargetAddress {
            message: "Invalid HOST header format (not UTF-8)".to_string(),
            uri: uri.to_string(),
            headers: headers_to_vec(headers),
        })?;

        match host_str.rsplit_once(':') {
            Some((_, port_str)) => {
                let port = port_str.parse::<u16>().unwrap_or(default_port);
                Ok(port.to_string())
            }
            None => Ok(default_port.to_string()),
        }
    } else {
        Err(Error::TargetAddress {
            message: "Missing port information in URI or Host header".to_string(),
            uri: uri.to_string(),
            headers: headers_to_vec(headers),
        })
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
        .map_err(|e| {
            Error::RequestBodyRead(
                RequestContextError {
                    method: parts.method.to_string(),
                    uri: parts.uri.to_string(),
                    version: format!("{:?}", parts.version),
                    headers: headers_to_vec(&parts.headers),
                    extensions: format!("{:?}", parts.extensions),
                    message: format!("{e:?}"),
                }
                .into(),
            )
        })?
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
        .map_err(|e| {
            Error::ResponseBodyRead(
                ResponseContextError {
                    status: parts.status.as_u16(),
                    version: format!("{:?}", parts.version),
                    headers: headers_to_vec(&parts.headers),
                    extensions: format!("{:?}", parts.extensions),
                    message: format!("{e:?}"),
                }
                .into(),
            )
        })?
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

fn fix_relative_uri(parts: &mut hyper::http::request::Parts, is_tls: bool) -> Result<(), Error> {
    if parts.uri.scheme().is_none()
        && let Some(host) = parts.headers.get(hyper::header::HOST)
    {
        let host = host.to_str().map_err(|_| {
            Error::FixRelativeUri(
                RequestContextError {
                    method: parts.method.to_string(),
                    uri: parts.uri.to_string(),
                    version: format!("{:?}", parts.version),
                    headers: headers_to_vec(&parts.headers),
                    extensions: format!("{:?}", parts.extensions),
                    message: "Invalid HOST header format (not UTF-8)".to_string(),
                }
                .into(),
            )
        })?;
        let scheme = if is_tls { "https" } else { "http" };
        let new_uri = format!("{}://{}{}", scheme, host, parts.uri);
        let uri = new_uri.parse::<Uri>().map_err(|e| {
            Error::FixRelativeUri(
                RequestContextError {
                    method: parts.method.to_string(),
                    uri: parts.uri.to_string(),
                    version: format!("{:?}", parts.version),
                    headers: headers_to_vec(&parts.headers),
                    extensions: format!("{:?}", parts.extensions),
                    message: format!("Failed to parse fixed URI: {e}"),
                }
                .into(),
            )
        })?;
        parts.uri = uri;
    }
    Ok(())
}
