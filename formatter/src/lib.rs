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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn format_non_code_text() {
        let input = "This is not JSON.";
        let output = format_code(input);
        assert_eq!(input, output);
    }

    #[rstest]
    fn format_valid_json() {
        let input = r#"{"name":"Alice","age":30,"city":"New York"}"#;
        let expected = r#"{
    "name": "Alice",
    "age": 30,
    "city": "New York"
}"#;

        let output = format_code(input);
        assert_eq!(expected, output);
    }

    #[rstest]
    fn format_invalid_json() {
        let input = r#"{"name":"Alice","age":30,"city":"New York}"#;
        let output = format_code(input);
        assert_eq!(input, output);
    }

    #[rstest]
    fn wrap_short_text() {
        let input = "Short text.";
        let output = wrap_text(input, 20);
        assert_eq!(input, output);
    }

    #[rstest]
    fn wrap_long_text() {
        let input = "This is a long text that should be wrapped to fit within the specified width.";
        let expected =
            "This is a long text that\nshould be wrapped to fit\nwithin the specified width.";

        let output = wrap_text(input, 30);
        assert_eq!(expected, output);
    }

    #[rstest]
    fn wrap_text_with_newlines() {
        let input =
            "This is a long text\nthat should be wrapped\nto fit within the specified\nwidth.";
        let expected =
            "This is a long text\nthat should be wrapped\nto fit within the specified\nwidth.";

        let output = wrap_text(input, 30);
        assert_eq!(expected, output);
    }

    #[rstest]
    fn wrap_text_with_long_word() {
        let input = "ThisIsAReallyLongWordThatExceedsTheWidthLimit";
        let expected = "ThisIsARea\nllyLongWor\ndThatExcee\ndsTheWidth\nLimit";

        let output = wrap_text(input, 10);
        assert_eq!(expected, output);
    }
}
