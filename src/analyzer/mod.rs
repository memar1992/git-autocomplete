pub mod files;
pub mod patterns;
pub mod scope;

use crate::git::Change;
use files::Kind;

/// Source changes take priority over their tests. Other mixed changes get a generic summary.
pub fn primary(changes: &[Change]) -> Kind {
    if changes
        .iter()
        .any(|c| files::classify(&c.name()) == Kind::Source)
    {
        return Kind::Source;
    }
    let kinds: Vec<_> = changes.iter().map(|c| files::classify(&c.name())).collect();
    match kinds.first() {
        Some(first) if kinds.iter().all(|k| k == first) => *first,
        _ => Kind::Other,
    }
}

/// Keep filenames readable, but never emit terminal controls or protocol delimiters.
pub fn label(value: &str) -> String {
    value
        .chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(
                    *c,
                    '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
                )
        })
        .take(48)
        .collect::<String>()
        .trim()
        .to_string()
}
