use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::{HeaderMap as HyperHeaderMap, Uri, body::Bytes};
use std::fmt::Display;

pub(crate) trait BytesExt {
    fn boxed(self) -> BoxBody<Bytes, Error>;
}

impl BytesExt for Bytes {
    fn boxed(self) -> BoxBody<Bytes, Error> {
        Full::new(self).map_err(|e| match e {}).boxed()
    }
}

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
    pub headers: HeaderMap,
    pub extensions: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResponseContextError {
    pub status: u16,
    pub version: String,
    pub headers: HeaderMap,
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
pub struct Header(String, String);

impl Header {
    pub fn key(&self) -> &str {
        &self.0
    }

    pub fn value(&self) -> &str {
        &self.1
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HeaderMap(HyperHeaderMap);

impl HeaderMap {
    pub fn new(headers: Vec<Header>) -> Self {
        Self(
            headers
                .into_iter()
                .fold(HyperHeaderMap::new(), |mut map, header| {
                    map.append(
                        header.key().parse().unwrap_or_else(|_| {
                            hyper::header::HeaderName::from_static("invalid-header-name")
                        }),
                        header.value().parse().unwrap_or_else(|_| {
                            hyper::header::HeaderValue::from_static("invalid-header-value")
                        }),
                    );
                    map
                }),
        )
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    pub fn get_all(&self, key: &str) -> Vec<&str> {
        self.0
            .get_all(key)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect()
    }
}

impl From<HyperHeaderMap> for HeaderMap {
    fn from(headers: HyperHeaderMap) -> Self {
        Self(headers)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Body(Vec<u8>);

impl Body {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Request {
    pub method: Method,
    pub url: Url,
    pub headers: HeaderMap,
    pub body: Body,
}

impl Request {
    pub(crate) async fn into_parts<B>(
        req: hyper::Request<B>,
    ) -> Result<(hyper::http::request::Parts, Bytes), Error>
    where
        B: hyper::body::Body,
        B::Error: std::fmt::Debug,
    {
        let (mut parts, body) = req.into_parts();
        clean_request_headers(&mut parts.headers);
        let body = Self::read_body(&parts, body).await?;
        Ok((parts, body))
    }

    pub(crate) async fn from_parts(
        parts: &hyper::http::request::Parts,
        body: BoxBody<Bytes, Error>,
    ) -> Result<Self, Error> {
        let body = Self::read_body(parts, body).await?;
        Ok(Self {
            method: parts.method.clone().into(),
            url: parts.uri.clone().into(),
            headers: parts.headers.clone().into(),
            body: Body::new(body.to_vec()),
        })
    }

    async fn read_body<B>(parts: &hyper::http::request::Parts, body: B) -> Result<Bytes, Error>
    where
        B: hyper::body::Body,
        B::Error: std::fmt::Debug,
    {
        let body = body
            .collect()
            .await
            .map_err(|e| {
                Error::RequestBodyRead(
                    RequestContextError {
                        method: parts.method.clone().into(),
                        uri: parts.uri.to_string(),
                        version: format!("{:?}", parts.version),
                        headers: parts.headers.clone().into(),
                        extensions: format!("{:?}", parts.extensions),
                        message: format!("{e:?}"),
                    }
                    .into(),
                )
            })?
            .to_bytes();
        Ok(body)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Response {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Body,
}

impl Response {
    pub(crate) async fn into_parts<B>(
        res: hyper::Response<B>,
    ) -> Result<(hyper::http::response::Parts, Bytes), Error>
    where
        B: hyper::body::Body,
        B::Error: std::fmt::Debug,
    {
        let (mut parts, body) = res.into_parts();
        clean_response_headers(&mut parts.headers);
        let body = Self::read_body(&parts, body).await?;
        Ok((parts, body))
    }

    pub(crate) async fn from_parts(
        parts: &hyper::http::response::Parts,
        body: BoxBody<Bytes, Error>,
    ) -> Result<Self, Error> {
        let body = Self::read_body(parts, body).await?;
        Ok(Self {
            status: parts.status.as_u16(),
            headers: parts.headers.clone().into(),
            body: Body::new(body.to_vec()),
        })
    }

    async fn read_body<B>(parts: &hyper::http::response::Parts, body: B) -> Result<Bytes, Error>
    where
        B: hyper::body::Body,
        B::Error: std::fmt::Debug,
    {
        let body = body
            .collect()
            .await
            .map_err(|e| {
                Error::ResponseBodyRead(
                    ResponseContextError {
                        status: parts.status.as_u16(),
                        version: format!("{:?}", parts.version),
                        headers: parts.headers.clone().into(),
                        extensions: format!("{:?}", parts.extensions),
                        message: format!("{e:?}"),
                    }
                    .into(),
                )
            })?
            .to_bytes();
        Ok(body)
    }
}

fn clean_request_headers(headers: &mut HyperHeaderMap) {
    headers.remove(hyper::header::CONNECTION);
    headers.remove("proxy-connection");
    headers.remove("keep-alive");
    headers.remove(hyper::header::TRANSFER_ENCODING);
}

fn clean_response_headers(headers: &mut HyperHeaderMap) {
    headers.remove(hyper::header::CONNECTION);
    headers.remove(hyper::header::TRANSFER_ENCODING);
}
