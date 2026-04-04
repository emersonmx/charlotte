use charlotte::{Request, RequestId, Response};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequestEntry {
    pub request_id: RequestId,
    pub request: Request,
    pub response: Option<Response>,
}
