//! What a plan's identity is, and what it deliberately ignores.
//!
//! An approval binds to a fingerprint over the semantic inputs, and an
//! apply regenerates it and refuses on any difference. So the fingerprint
//! has to answer one question exactly: would this plan still do the same
//! thing to the same target? Anything that changes the answer is in.
//! Anything that cannot is out, or every rerun would invalidate itself.
//!
//! The encoding is RFC 8785 JSON canonicalization, restricted to the value
//! kinds a plan's inputs need: integers, strings, booleans, nulls, arrays,
//! and string-keyed objects. Floating point is excluded by the type rather
//! than handled, because the one genuinely subtle part of the RFC is the
//! number grammar and a plan has no reason to carry a float.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::ownership::Sha256;

/// The value kinds a fingerprint input may be.
///
/// Closed on purpose. A kind added here is a kind the canonical encoding
/// has to be correct for, and the restriction is what keeps that provable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// A JSON null.
    Null,
    /// A JSON boolean.
    Bool(bool),
    /// A JSON integer. No float, ever.
    Int(i64),
    /// A JSON string.
    Text(String),
    /// A JSON array, in the order the plan produced it.
    List(Vec<Self>),
    /// A JSON object, keyed by string and sorted at encoding time.
    Map(BTreeMap<String, Self>),
}

impl Value {
    /// A map built from pairs, for the planner's own construction.
    #[must_use]
    pub fn map<I: IntoIterator<Item = (&'static str, Self)>>(pairs: I) -> Self {
        Self::Map(
            pairs
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    /// A string value.
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    /// A string value, or null where there is none.
    #[must_use]
    pub fn maybe(value: Option<impl Into<String>>) -> Self {
        value.map_or(Self::Null, |held| Self::Text(held.into()))
    }
}

/// Encode one value as RFC 8785 canonical JSON.
///
/// Keys sort by their UTF-16 code units, which is what the RFC says and
/// what a plain byte sort gets wrong for anything outside the basic
/// multilingual plane.
#[must_use]
pub fn canonical(value: &Value) -> String {
    let mut out = String::new();
    encode(value, &mut out);
    out
}

fn encode(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(number) => out.push_str(&number.to_string()),
        Value::Text(text) => encode_string(text, out),
        Value::List(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                encode(item, out);
            }
            out.push(']');
        }
        Value::Map(entries) => {
            let mut keys: Vec<&String> = entries.keys().collect();
            keys.sort_by_key(|left| utf16_units(left));
            out.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                encode_string(key, out);
                out.push(':');
                if let Some(held) = entries.get(*key) {
                    encode(held, out);
                }
            }
            out.push('}');
        }
    }
}

fn utf16_units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// Serialize a string as the RFC's escaping rules require.
///
/// The shortest form wins: the two-character escapes where one exists, a
/// `\u` escape for every other control character, and the character itself
/// everywhere else.
fn encode_string(text: &str, out: &mut String) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control < '\u{20}' => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", control as u32);
            }
            ordinary => out.push(ordinary),
        }
    }
    out.push('"');
}

/// The digest of one canonical encoding.
#[must_use]
pub fn fingerprint(value: &Value) -> Sha256 {
    Sha256::of(canonical(value).as_bytes())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    #[test]
    fn the_scalar_forms_encode_as_the_rfc_writes_them() {
        assert_eq!(canonical(&Value::Null), "null");
        assert_eq!(canonical(&Value::Bool(true)), "true");
        assert_eq!(canonical(&Value::Bool(false)), "false");
        assert_eq!(canonical(&Value::Int(0)), "0");
        assert_eq!(canonical(&Value::Int(-1)), "-1");
        assert_eq!(
            canonical(&Value::Int(9_007_199_254_740_991)),
            "9007199254740991"
        );
    }

    /// The RFC's own string vectors: the two-character escapes, a control
    /// character with no short form, and a character that must stay as it
    /// is rather than being escaped.
    #[test]
    fn strings_escape_exactly_what_the_rfc_escapes() {
        assert_eq!(canonical(&Value::text("")), "\"\"");
        assert_eq!(canonical(&Value::text("a\"b")), "\"a\\\"b\"");
        assert_eq!(canonical(&Value::text("a\\b")), "\"a\\\\b\"");
        assert_eq!(
            canonical(&Value::text("\u{8}\u{c}\n\r\t")),
            "\"\\b\\f\\n\\r\\t\""
        );
        assert_eq!(canonical(&Value::text("\u{1}")), "\"\\u0001\"");
        assert_eq!(canonical(&Value::text("\u{7f}")), "\"\u{7f}\"");
        assert_eq!(canonical(&Value::text("é")), "\"é\"");
        assert_eq!(canonical(&Value::text("😀")), "\"😀\"");
    }

    /// The RFC sorts keys by UTF-16 code unit, which puts a character
    /// above the basic multilingual plane below one that a byte sort would
    /// place after it.
    #[test]
    fn keys_sort_by_utf16_code_unit() {
        let held = Value::map([("\u{fb33}", Value::Int(1)), ("😀", Value::Int(2))]);
        // U+1F600 encodes as the surrogate pair D83D DE00, whose first
        // unit is below U+FB33, so the emoji sorts first.
        assert_eq!(canonical(&held), "{\"😀\":2,\"\u{fb33}\":1}");
    }

    #[test]
    fn map_insertion_order_cannot_change_the_digest() {
        let one = Value::map([
            ("b", Value::Int(2)),
            ("a", Value::Int(1)),
            ("c", Value::text("three")),
        ]);
        let two = Value::map([
            ("c", Value::text("three")),
            ("a", Value::Int(1)),
            ("b", Value::Int(2)),
        ]);
        assert_eq!(canonical(&one), canonical(&two));
        assert_eq!(fingerprint(&one), fingerprint(&two));
        assert_eq!(canonical(&one), "{\"a\":1,\"b\":2,\"c\":\"three\"}");
    }

    #[test]
    fn list_order_does_change_the_digest() {
        let one = Value::List(vec![Value::Int(1), Value::Int(2)]);
        let two = Value::List(vec![Value::Int(2), Value::Int(1)]);
        assert_ne!(fingerprint(&one), fingerprint(&two));
    }

    #[test]
    fn nesting_encodes_without_whitespace() {
        let held = Value::map([(
            "outer",
            Value::map([("inner", Value::List(vec![Value::Null]))]),
        )]);
        assert_eq!(canonical(&held), "{\"outer\":{\"inner\":[null]}}");
    }

    #[test]
    fn a_maybe_value_is_null_where_there_is_nothing() {
        assert_eq!(canonical(&Value::maybe(None::<String>)), "null");
        assert_eq!(canonical(&Value::maybe(Some("x"))), "\"x\"");
    }
}
