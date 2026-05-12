use serde::Serialize;
use serde_json::ser::{PrettyFormatter, Serializer};

const DEFAULT_INDENT: &[u8] = b"    ";

pub fn format(code: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(code) {
        let mut buf = Vec::new();
        let formatter = PrettyFormatter::with_indent(DEFAULT_INDENT);
        let mut ser = Serializer::with_formatter(&mut buf, formatter);
        if json.serialize(&mut ser).is_ok() {
            return String::from_utf8(buf).unwrap_or_else(|_| code.to_string());
        }
    }

    code.to_string()
}
