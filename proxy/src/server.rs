use crate::{
    certs::CertificateStore,
    http::{
        self, BytesExt, ClientBuilder, HeaderMap, HyperRequest, HyperResponse, IncomingRequest,
        IncomingResponse, Request, RequestContextError, Response, ServerBuilder,
    },
};
use http_body_util::{BodyExt, Empty};
use hyper::{
    HeaderMap as HyperHeaderMap, Uri, body::Bytes, header::HeaderValue, service::service_fn,
    upgrade::Upgraded,
};
use hyper_util::rt::TokioIo;
use rustls::pki_types::{DnsName, ServerName};
use std::{
    fmt::Display,
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
        headers: HeaderMap,
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

    #[error("Failed to read or process HTTP message: {0}")]
    Http(#[source] http::Error),

    #[error("Failed to send message: {message:?}, error: {source}")]
    Channel {
        message: Box<Message>,
        #[source]
        source: BoxError,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(usize);

impl RequestId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

impl Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
            &hyper::Method::CONNECT => self.handle_connect(req, client_addr).await,
            _ => self.handle_regular(req, client_addr, false).await,
        }
    }

    async fn handle_connect(
        self: Arc<Self>,
        req: IncomingRequest,
        client_addr: std::net::SocketAddr,
    ) -> Result<HyperResponse, Error> {
        let request_id = self.request_id_counter.fetch_add(1, Ordering::Relaxed);

        let (parts, body) = Request::into_parts(req).await.map_err(Error::Http)?;
        let request = Request::from_parts(&parts, body.clone().boxed())
            .await
            .map_err(Error::Http)?;
        let _ = self
            .message_channel
            .send(Message::RequestSent((RequestId::new(request_id), request)))
            .await;

        let req = HyperRequest::from_parts(parts, body.boxed());

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
                                .send(Message::ErrorOccurred((
                                    Some(RequestId::new(request_id)),
                                    e.into(),
                                )))
                                .await;
                        }
                    }
                    Err(e) => {
                        let _ = message_channel
                            .send(Message::ErrorOccurred((
                                Some(RequestId::new(request_id)),
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

        let (parts, body) = Response::into_parts(res).await.map_err(Error::Http)?;
        let response = Response::from_parts(&parts, body.clone().boxed())
            .await
            .map_err(Error::Http)?;
        let _ = self
            .message_channel
            .send(Message::ResponseReceived((
                RequestId::new(request_id),
                response,
            )))
            .await;

        let res = HyperResponse::from_parts(parts, body.boxed());
        Ok(res)
    }

    async fn handle_upgraded(
        self: Arc<Self>,
        upgraded: Upgraded,
        addr: &str,
        client_addr: std::net::SocketAddr,
    ) -> Result<(), Error> {
        let request_id = self.request_id_counter.load(Ordering::Relaxed);

        let domain = addr.split(':').next().unwrap_or("127.0.0.1");
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
                        Some(RequestId::new(request_id)),
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

        let (mut parts, body) = Request::into_parts(req).await.map_err(Error::Http)?;
        fix_relative_uri(&mut parts, is_tls)?;
        let request = Request::from_parts(&parts, body.clone().boxed())
            .await
            .map_err(Error::Http)?;
        let _ = self
            .message_channel
            .send(Message::RequestSent((RequestId::new(request_id), request)))
            .await;

        let self_clone = self.clone();
        let res = self_clone
            .fetch(HyperRequest::from_parts(parts, body.boxed()))
            .await?;

        let (parts, body) = Response::into_parts(res).await.map_err(Error::Http)?;
        let response = Response::from_parts(&parts, body.clone().boxed())
            .await
            .map_err(Error::Http)?;
        let _ = self
            .message_channel
            .send(Message::ResponseReceived((
                RequestId::new(request_id),
                response,
            )))
            .await;

        Ok(HyperResponse::from_parts(parts, body.boxed()))
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

fn get_target_addr(uri: &Uri, headers: &HyperHeaderMap) -> Result<String, Error> {
    let host = get_host(uri, headers)?;
    let port = get_port(uri, headers)?;
    Ok(format!("{host}:{port}"))
}

fn get_host(uri: &Uri, headers: &HyperHeaderMap) -> Result<String, Error> {
    let headers: HeaderMap = headers.clone().into();
    if let Some(host) = uri.host() {
        Ok(host.to_string())
    } else if let Some(host) = headers.get(hyper::header::HOST.as_str()) {
        match host.rsplit_once(':') {
            Some((host, _)) => Ok(host.to_string()),
            None => Ok(host.to_string()),
        }
    } else {
        Err(Error::TargetAddress {
            message: "Missing host information in URI or Host header".to_string(),
            uri: uri.to_string(),
            headers,
        })
    }
}

fn get_port(uri: &Uri, headers: &HyperHeaderMap) -> Result<String, Error> {
    let headers: HeaderMap = headers.clone().into();
    let default_port = match uri.scheme_str() {
        Some("https") => 443_u16,
        _ => 80_u16,
    };

    if uri.host().is_some() {
        let port = uri.port_u16().unwrap_or(default_port);
        Ok(port.to_string())
    } else if let Some(host) = headers.get(hyper::header::HOST.as_str()) {
        match host.rsplit_once(':') {
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
            headers,
        })
    }
}

fn fix_relative_uri(parts: &mut hyper::http::request::Parts, is_tls: bool) -> Result<(), Error> {
    if parts.uri.scheme().is_none()
        && let Some(host) = parts.headers.get(hyper::header::HOST)
    {
        let headers: HeaderMap = parts.headers.clone().into();
        let host = host.to_str().map_err(|_| {
            Error::FixRelativeUri(
                RequestContextError {
                    method: parts.method.clone().into(),
                    uri: parts.uri.to_string(),
                    version: format!("{:?}", parts.version),
                    headers: headers.clone(),
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
                    method: parts.method.clone().into(),
                    uri: parts.uri.to_string(),
                    version: format!("{:?}", parts.version),
                    headers,
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
