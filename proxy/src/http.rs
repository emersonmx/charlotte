use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::{HeaderMap as HyperHeaderMap, Uri, body::Bytes};
use std::{collections::HashMap, fmt::Display};

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
pub struct HeaderMap {
    headers: Vec<Header>,
    index_map: HashMap<String, Vec<usize>>,
}

impl HeaderMap {
    pub fn new(headers: Vec<Header>) -> Self {
        let mut index_map = HashMap::new();
        for (i, header) in headers.iter().enumerate() {
            index_map
                .entry(header.key().to_lowercase())
                .or_insert_with(Vec::new)
                .push(i);
        }
        Self { headers, index_map }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.index_map
            .get(&key.to_lowercase())
            .and_then(|indices| indices.first())
            .map(|&i| self.headers[i].value())
    }

    pub fn get_all(&self, key: &str) -> Vec<&str> {
        self.index_map
            .get(&key.to_lowercase())
            .map(|indices| indices.iter().map(|&i| self.headers[i].value()).collect())
            .unwrap_or_default()
    }
}

impl From<HyperHeaderMap> for HeaderMap {
    fn from(headers: HyperHeaderMap) -> Self {
        let headers_vec = headers
            .iter()
            .map(|(k, v)| {
                Header(
                    k.as_str().to_string(),
                    v.to_str()
                        .unwrap_or("<invalid UTF-8 in header value>")
                        .to_string(),
                )
            })
            .collect();
        Self::new(headers_vec)
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
        let headers: HeaderMap = parts.headers.clone().into();
        let body = Self::read_body(parts, body).await?;
        Ok(Self {
            method: parts.method.clone().into(),
            url: parts.uri.clone().into(),
            headers,
            body: Body::new(body.to_vec()),
        })
    }

    async fn read_body<B>(parts: &hyper::http::request::Parts, body: B) -> Result<Bytes, Error>
    where
        B: hyper::body::Body,
        B::Error: std::fmt::Debug,
    {
        let headers: HeaderMap = parts.headers.clone().into();
        let body = body
            .collect()
            .await
            .map_err(|e| {
                Error::RequestBodyRead(
                    RequestContextError {
                        method: parts.method.clone().into(),
                        uri: parts.uri.to_string(),
                        version: format!("{:?}", parts.version),
                        headers: headers.clone(),
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
        let headers: HeaderMap = parts.headers.clone().into();
        let body = Self::read_body(parts, body).await?;
        Ok(Self {
            status: parts.status.as_u16(),
            headers,
            body: Body::new(body.to_vec()),
        })
    }

    async fn read_body<B>(parts: &hyper::http::response::Parts, body: B) -> Result<Bytes, Error>
    where
        B: hyper::body::Body,
        B::Error: std::fmt::Debug,
    {
        let headers: HeaderMap = parts.headers.clone().into();
        let body = body
            .collect()
            .await
            .map_err(|e| {
                Error::ResponseBodyRead(
                    ResponseContextError {
                        status: parts.status.as_u16(),
                        version: format!("{:?}", parts.version),
                        headers: headers.clone(),
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
