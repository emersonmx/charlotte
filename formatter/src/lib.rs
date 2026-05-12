use serde::Serialize;
use serde_json::ser::{PrettyFormatter, Serializer};

const DEFAULT_INDENT: &[u8] = b"    ";

pub fn format_code(text: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
        let mut buffer = Vec::new();
        let formatter = PrettyFormatter::with_indent(DEFAULT_INDENT);
        let mut serializer = Serializer::with_formatter(&mut buffer, formatter);
        if json.serialize(&mut serializer).is_ok() {
            return String::from_utf8(buffer).unwrap_or_else(|_| text.to_string());
        }
    }

    text.to_string()
}

pub fn wrap_text(text: &str, max_width: usize) -> String {
    textwrap::fill(text, max_width)
}
