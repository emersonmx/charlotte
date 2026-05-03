pub(crate) use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
pub(crate) use hyper::body::Bytes;
use hyper::{HeaderMap as HyperHeaderMap, Uri};
use std::fmt::Display;

pub(crate) type ClientBuilder = hyper::client::conn::http1::Builder;
pub(crate) type ServerBuilder = hyper::server::conn::http1::Builder;
pub(crate) type IncomingRequest = hyper::Request<hyper::body::Incoming>;
pub(crate) type IncomingResponse = hyper::Response<hyper::body::Incoming>;
pub(crate) type HyperRequest = hyper::Request<BoxBody<Bytes, Error>>;
pub(crate) type HyperResponse = hyper::Response<BoxBody<Bytes, Error>>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to read request body: {0:?}")]
    RequestBodyRead(Box<RequestContextError>),
    #[error("Failed to read response body: {0:?}")]
    ResponseBodyRead(Box<ResponseContextError>),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequestContextError {
    pub method: Method,
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum Method {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Connect,
    Trace,
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let method_str = match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Connect => "CONNECT",
            Method::Trace => "TRACE",
        };
        write!(f, "{method_str}")
    }
}

impl From<hyper::Method> for Method {
    fn from(method: hyper::Method) -> Self {
        match method {
            hyper::Method::GET => Method::Get,
            hyper::Method::POST => Method::Post,
            hyper::Method::PUT => Method::Put,
            hyper::Method::PATCH => Method::Patch,
            hyper::Method::DELETE => Method::Delete,
            hyper::Method::HEAD => Method::Head,
            hyper::Method::OPTIONS => Method::Options,
            hyper::Method::CONNECT => Method::Connect,
            hyper::Method::TRACE => Method::Trace,
            _ => Method::default(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: Option<String>,
}

impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let port_part = if let Some(port) = self.port {
            format!(":{port}")
        } else {
            "".to_string()
        };
        let query_part = if let Some(query) = &self.query {
            format!("?{query}")
        } else {
            "".to_string()
        };
        write!(
            f,
            "{}://{}{}{}{}",
            self.scheme, self.host, port_part, self.path, query_part
        )
    }
}

impl From<Uri> for Url {
    fn from(uri: Uri) -> Self {
        let scheme = uri.scheme_str().unwrap_or("http").to_string();
        let host = uri.host().unwrap_or("127.0.0.1").to_string();
        let port = uri.port_u16();
        let path = uri.path().to_string();
        let query = uri.query().map(|q| q.to_string());

        Self {
            scheme,
            host,
            port,
            path,
            query,
        }
    }
}

impl From<&str> for Url {
    fn from(s: &str) -> Self {
        let uri = s.parse::<Uri>().unwrap_or_else(|_| Uri::from_static("/"));
        Self::from(uri)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Request {
    pub method: Method,
    pub url: Url,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub(crate) async fn from_parts(
        parts: &hyper::http::request::Parts,
        body: BoxBody<Bytes, Error>,
    ) -> Result<Self, Error> {
        let body = body
            .collect()
            .await
            .map_err(|e| {
                Error::RequestBodyRead(
                    RequestContextError {
                        method: parts.method.clone().into(),
                        uri: parts.uri.to_string(),
                        version: format!("{:?}", parts.version),
                        headers: headers_to_vec(&parts.headers),
                        extensions: format!("{:?}", parts.extensions),
                        message: format!("{e:?}"),
                    }
                    .into(),
                )
            })?
            .to_bytes()
            .to_vec();

        Ok(Self {
            method: parts.method.clone().into(),
            url: parts.uri.clone().into(),
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
    pub async fn from_parts(
        parts: &hyper::http::response::Parts,
        body: BoxBody<Bytes, Error>,
    ) -> Result<Self, Error> {
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
            .to_bytes()
            .to_vec();

        Ok(Self {
            status: parts.status.as_u16(),
            headers: headers_to_vec(&parts.headers),
            body,
        })
    }
}

pub(crate) fn boxed_body_from_bytes(body: Bytes) -> BoxBody<Bytes, Error> {
    Full::new(body).map_err(|e| match e {}).boxed()
}

pub(crate) fn headers_to_vec(headers: &HyperHeaderMap) -> Vec<(String, String)> {
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

pub(crate) async fn extract_request_parts<B>(
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
                    method: parts.method.clone().into(),
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

fn clean_request_headers(headers: &mut HyperHeaderMap) {
    headers.remove(hyper::header::CONNECTION);
    headers.remove("proxy-connection");
    headers.remove("keep-alive");
    headers.remove(hyper::header::TRANSFER_ENCODING);
}

pub(crate) async fn extract_response_parts<B>(
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

fn clean_response_headers(headers: &mut HyperHeaderMap) {
    headers.remove(hyper::header::CONNECTION);
    headers.remove(hyper::header::TRANSFER_ENCODING);
}
