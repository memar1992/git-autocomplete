pub mod analyzer;
pub mod cli;
pub mod generator;
pub mod git;

/// Encode JSON without a serialization dependency. All public output uses this encoder.
pub fn json_string(value: &str) -> String {
    let mut result = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => result.push_str(&format!("\\u{:04x}", c as u32)),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn json_escapes_untrusted_strings() {
        assert_eq!(
            super::json_string("\"\\\n\t\0雪"),
            "\"\\\"\\\\\\n\\t\\u0000雪\""
        );
    }
}
