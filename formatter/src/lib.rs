use serde::Serialize;
use serde_json::ser::{PrettyFormatter, Serializer};

const DEFAULT_INDENT: &[u8] = b"    ";

pub fn format_code(text: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
        let mut buf = Vec::new();
        let formatter = PrettyFormatter::with_indent(DEFAULT_INDENT);
        let mut ser = Serializer::with_formatter(&mut buf, formatter);
        if json.serialize(&mut ser).is_ok() {
            return String::from_utf8(buf).unwrap_or_else(|_| text.to_string());
        }
    }

    text.to_string()
}

pub fn wrap_text(text: &str, max_width: usize) -> String {
    textwrap::fill(text, max_width)
}
