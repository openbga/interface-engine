//! Typed text values (configuration.md §7.7): the type of a PDF report field, checked when a
//! report is parsed. Dates use the formats of the `datetime` rule ([`crate::datetime`]) and
//! decimals the grammar of the numeric filters ([`crate::decimal`]).

use crate::datetime::DateFormat;
use crate::decimal::Decimal;

/// Most formats a date or datetime value may have; the first that matches wins.
const MAX_FORMATS: usize = 4;

/// The type of a field's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueType(Kind);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Text,
    Integer,
    Decimal,
    Temporal(Vec<DateFormat>),
}

impl ValueType {
    /// Compiles a configured `type` (`string`, `integer`, `decimal`, `date`, `datetime`)
    /// with its formats: 1–4 for `date` (year, month and day) and `datetime` (also hour and
    /// minute), none for the others.
    ///
    /// # Errors
    /// A sentence saying what is wrong with the type or a format.
    pub fn compile(kind: &str, formats: &[String]) -> Result<Self, String> {
        let temporal = match kind {
            "string" | "integer" | "decimal" if !formats.is_empty() => {
                return Err(format!("`format` applies to date and datetime, not {kind}"));
            }
            "string" => return Ok(Self(Kind::Text)),
            "integer" => return Ok(Self(Kind::Integer)),
            "decimal" => return Ok(Self(Kind::Decimal)),
            "date" => "ydM".chars().collect::<Vec<_>>(),
            "datetime" => "ydMHm".chars().collect(),
            _ => return Err("expected string, integer, decimal, date or datetime".to_owned()),
        };
        if formats.is_empty() || formats.len() > MAX_FORMATS {
            return Err(format!("{kind} needs 1 to {MAX_FORMATS} formats"));
        }
        let compiled = formats
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let format = DateFormat::parse(text)
                    .map_err(|e| format!("format {i}: {} at position {}", e.reason, e.position))?;
                match temporal.iter().find(|c| !format.has(**c)) {
                    Some(missing) => Err(format!(
                        "format {i} lacks the `{missing}` component a {kind} needs"
                    )),
                    None => Ok(format),
                }
            })
            .collect::<Result<_, String>>()?;
        Ok(Self(Kind::Temporal(compiled)))
    }

    /// Whether `text` is a value of this type. Empty text always is: an absent value is
    /// empty, whatever its type.
    #[must_use]
    pub fn accepts(&self, text: &str) -> bool {
        if text.is_empty() {
            return true;
        }
        match &self.0 {
            Kind::Text => true,
            Kind::Integer => {
                let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
                !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
            }
            Kind::Decimal => text.trim() == text && Decimal::parse(text).is_some(),
            Kind::Temporal(formats) => formats.iter().any(|f| f.read(text).is_ok()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile(kind: &str, formats: &[&str]) -> Result<ValueType, String> {
        let formats: Vec<String> = formats.iter().map(|f| (*f).to_owned()).collect();
        ValueType::compile(kind, &formats)
    }

    #[test]
    fn numbers() {
        let integer = compile("integer", &[]).unwrap();
        assert!(integer.accepts("1902") && integer.accepts("-3") && integer.accepts(""));
        assert!(!integer.accepts("19.0") && !integer.accepts("1 902") && !integer.accepts("-"));
        let decimal = compile("decimal", &[]).unwrap();
        assert!(decimal.accepts("38.5") && decimal.accepts("-.5"));
        assert!(!decimal.accepts("1e3") && !decimal.accepts("<5") && !decimal.accepts(" 1"));
    }

    #[test]
    fn dates_try_each_format() {
        let dob = compile("datetime", &["M/d/yyyy H:mm", "M/d/yyyy"]).unwrap_err();
        assert!(dob.contains("format 1 lacks the `H`"), "{dob}");
        let dob = compile("datetime", &["M/d/yyyy H:mm", "yyyy-MM-dd'T'HH:mm"]).unwrap();
        assert!(dob.accepts("1/2/2026 8:05") && dob.accepts("2026-01-02T08:05"));
        assert!(!dob.accepts("1/2/2026") && !dob.accepts("2/30/2026 8:05"));
        let date = compile("date", &["M/d/yyyy"]).unwrap();
        assert!(date.accepts("12/31/2026"));
    }

    #[test]
    fn configuration_problems() {
        assert!(compile("date", &[]).unwrap_err().contains("1 to 4 formats"));
        assert!(compile("date", &["yyyy", "MM", "dd", "HH", "mm"]).is_err());
        assert!(
            compile("decimal", &["yyyy"])
                .unwrap_err()
                .contains("not decimal")
        );
        assert!(compile("number", &[]).is_err());
        assert!(
            compile("date", &["yyy-MM-dd"])
                .unwrap_err()
                .starts_with("format 0: unknown token")
        );
    }
}
