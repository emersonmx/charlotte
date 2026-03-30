#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequestEntry {
    pub request_id: usize,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
