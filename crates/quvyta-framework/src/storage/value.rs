//! Setting values, the typed conversions applications use, and writing them as TOML.

use std::fmt::Write as _;

/// A value stored in settings.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    /// `true` or `false`.
    Bool(bool),
    /// A whole number.
    Integer(i64),
    /// A number with a fraction.
    Float(f64),
    /// Text.
    Text(String),
    /// A list of values.
    List(Vec<SettingValue>),
}

impl SettingValue {
    /// The TOML type name, for diagnostics.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Bool(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::Float(_) => "float",
            Self::Text(_) => "string",
            Self::List(_) => "array",
        }
    }

    /// The value as it is written in a file, for diagnostics.
    pub(crate) fn literal(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    /// Writes the value as TOML.
    pub(crate) fn write(&self, out: &mut String) {
        // Writing into a `String` cannot fail, so there is no error here to carry anywhere; the
        // results are dropped for that reason and no other.
        match self {
            Self::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Self::Integer(value) => {
                let _ = write!(out, "{value}");
            }
            Self::Float(value) => out.push_str(&float(*value)),
            Self::Text(text) => quote(text, out),
            Self::List(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push_str(", ");
                    }
                    item.write(out);
                }
                out.push(']');
            }
        }
    }
}

fn float(value: f64) -> String {
    if value.is_nan() {
        "nan".to_owned()
    } else if value.is_infinite() {
        if value > 0.0 { "inf".to_owned() } else { "-inf".to_owned() }
    } else {
        // `Debug` always keeps a fraction or an exponent, which TOML needs to read a float back.
        format!("{value:?}")
    }
}

/// Writes `text` as a TOML basic string.
pub(crate) fn quote(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // As in `write` above: the target is a `String`, which has no failure to report.
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Writes one key segment: bare when TOML allows it, quoted otherwise.
pub(crate) fn key(segment: &str, out: &mut String) {
    let bare = !segment.is_empty() && segment.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if bare {
        out.push_str(segment);
    } else {
        quote(segment, out);
    }
}

/// A Rust type that can be read from and written to settings.
///
/// Implemented for `bool`, `String`, the integer types, `f32`, `f64` and `Vec<String>`. Integers
/// are stored as `i64`; a `u64` or `usize` above `i64::MAX` is stored as `i64::MAX`.
pub trait Setting: Sized {
    /// The stored form.
    fn to_setting(self) -> SettingValue;

    /// Reads a stored value; `None` when it has another type or does not fit.
    fn from_setting(value: &SettingValue) -> Option<Self>;
}

impl Setting for bool {
    fn to_setting(self) -> SettingValue {
        SettingValue::Bool(self)
    }

    fn from_setting(value: &SettingValue) -> Option<Self> {
        match value {
            SettingValue::Bool(value) => Some(*value),
            _ => None,
        }
    }
}

impl Setting for String {
    fn to_setting(self) -> SettingValue {
        SettingValue::Text(self)
    }

    fn from_setting(value: &SettingValue) -> Option<Self> {
        match value {
            SettingValue::Text(text) => Some(text.clone()),
            _ => None,
        }
    }
}

impl Setting for f64 {
    fn to_setting(self) -> SettingValue {
        SettingValue::Float(self)
    }

    fn from_setting(value: &SettingValue) -> Option<Self> {
        match value {
            SettingValue::Float(value) => Some(*value),
            // A whole number written without a fraction is still a number the user meant.
            SettingValue::Integer(value) => i32::try_from(*value).ok().map(f64::from),
            _ => None,
        }
    }
}

impl Setting for f32 {
    fn to_setting(self) -> SettingValue {
        SettingValue::Float(f64::from(self))
    }

    fn from_setting(value: &SettingValue) -> Option<Self> {
        // Settings hold small numbers such as ratios; precision beyond f32 is not meaningful.
        f64::from_setting(value).map(|value| value as f32)
    }
}

macro_rules! integer_setting {
    ($($ty:ty),*) => {$(
        impl Setting for $ty {
            fn to_setting(self) -> SettingValue {
                SettingValue::Integer(i64::try_from(self).unwrap_or(i64::MAX))
            }

            fn from_setting(value: &SettingValue) -> Option<Self> {
                match value {
                    SettingValue::Integer(value) => <$ty>::try_from(*value).ok(),
                    _ => None,
                }
            }
        }
    )*};
}

integer_setting!(i8, i16, i32, i64, u8, u16, u32, u64, usize);

impl Setting for Vec<String> {
    fn to_setting(self) -> SettingValue {
        SettingValue::List(self.into_iter().map(SettingValue::Text).collect())
    }

    fn from_setting(value: &SettingValue) -> Option<Self> {
        match value {
            SettingValue::List(items) => items.iter().map(String::from_setting).collect(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_toml_literals() {
        assert_eq!(SettingValue::Text("a \"b\"\n\\".into()).literal(), r#""a \"b\"\n\\""#);
        assert_eq!(SettingValue::Float(1.0).literal(), "1.0");
        assert_eq!(SettingValue::Float(f64::NAN).literal(), "nan");
        assert_eq!(vec!["x".to_owned(), "y".to_owned()].to_setting().literal(), r#"["x", "y"]"#);
        let mut out = String::new();
        key("tab width", &mut out);
        key("tab-width", &mut out);
        assert_eq!(out, "\"tab width\"tab-width");
    }

    #[test]
    fn typed_reads_check_types_and_ranges() {
        assert_eq!(u8::from_setting(&SettingValue::Integer(300)), None);
        assert_eq!(u16::from_setting(&SettingValue::Integer(300)), Some(300));
        assert_eq!(f64::from_setting(&SettingValue::Integer(2)), Some(2.0));
        assert_eq!(bool::from_setting(&SettingValue::Text("true".into())), None);
    }
}
