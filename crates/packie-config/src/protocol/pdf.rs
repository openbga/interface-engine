//! PDF report settings (`docs/spec/configuration.md` §7.7): the template that locates
//! labelled fields (`docs/spec/pdf-reports.md` §4).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::ColumnType;

/// Where a field's value sits relative to its label.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum FieldPosition {
    /// On the label's line.
    #[default]
    Right,
    /// In the block of lines under the label.
    Below,
}

/// One text, or a list of texts (alternative labels, date formats).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(untagged)]
pub enum OneOrMore {
    /// A single text.
    One(String),
    /// Several texts; the first that applies wins.
    More(Vec<String>),
}

impl OneOrMore {
    /// The texts in order.
    #[must_use]
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Self::One(text) => vec![text.clone()],
            Self::More(texts) => texts.clone(),
        }
    }
}

/// A labelled field of the template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct PdfField {
    /// `^[a-z][a-z0-9_]{0,40}$`, unique; addressed as `field.<name>`. `control_id`,
    /// `sample_id` and `patient_id` fill the message identifiers.
    pub name: String,
    /// The label, or 1–8 alternatives (1–100 characters each).
    pub label: OneOrMore,
    /// Default `right`.
    #[serde(default)]
    pub position: FieldPosition,
    /// Which match of the label on a page: 0 the first, −1 the last; −1 to 20. Default 0.
    #[serde(default)]
    pub occurrence: i32,
    /// Read on page 1 only; the label on later pages is ignored. Default `false`.
    #[serde(default)]
    pub first_page_only: bool,
    /// `below` only: the block line, 1–20. Default 1.
    #[serde(default)]
    pub line: Option<u32>,
    /// `below` only: the phrase of that line, 0 the first, −1 the last; −1 to 20. Default 0.
    #[serde(default)]
    pub phrase: Option<i32>,
    /// `below` only: how far right of the label a block line may start, 0–600 points.
    /// Default 4.
    #[serde(default, with = "rust_decimal::serde::float_option")]
    #[ts(type = "number | null")]
    pub indent: Option<Decimal>,
    /// The label must be on page 1 with a value. Default `false`.
    #[serde(default)]
    pub required: bool,
    /// Default `string`.
    #[serde(default, rename = "type")]
    pub field_type: ColumnType,
    /// Required for `date` and `datetime`: a format or 1–4 formats.
    #[serde(default)]
    pub format: Option<OneOrMore>,
}

/// PDF report settings (§7.7). Source only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct PdfReportSettings {
    /// A larger file is not read; 1024–268435456. Default 20 MiB.
    #[serde(default = "mib_20")]
    pub max_file_bytes: u64,
    /// 1–500. Default 20.
    #[serde(default = "twenty")]
    pub max_pages: u32,
    /// A gap of at least this many times the font size separates phrases; 0.5–20, at most
    /// two decimal places. Default 2.
    #[serde(default = "two", with = "rust_decimal::serde::float")]
    #[ts(type = "number")]
    pub phrase_gap: Decimal,
    /// 0–8 texts; the label area ends at the first line whose first phrase begins with one.
    #[serde(default)]
    pub label_area_end: Vec<String>,
    /// 1–200 fields.
    pub fields: Vec<PdfField>,
}

impl PdfReportSettings {
    /// The field names, in template order.
    pub fn field_names(&self) -> impl Iterator<Item = &str> {
        self.fields.iter().map(|f| f.name.as_str())
    }
}

const fn mib_20() -> u64 {
    20 * 1024 * 1024
}
const fn twenty() -> u32 {
    20
}
const fn two() -> Decimal {
    Decimal::TWO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_formats_take_one_text_or_a_list() {
        let json = r#"{"fields":[
            {"name":"dob","label":"DOB:","type":"datetime","format":["M/d/yyyy HH:mm","M/d/yyyy"]},
            {"name":"code","label":["Physician/Clinic","Physician:"],"position":"below","phrase":1,"indent":60}
        ]}"#;
        let s: PdfReportSettings = serde_json::from_str(json).unwrap();
        assert_eq!(s.phrase_gap, Decimal::TWO);
        assert_eq!(s.fields[0].label.to_vec(), ["DOB:"]);
        assert_eq!(
            s.fields[0]
                .format
                .as_ref()
                .map(OneOrMore::to_vec)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(s.fields[1].label.to_vec().len(), 2);
        assert_eq!(s.fields[1].indent, Some(Decimal::from(60)));
        assert_eq!(s.fields[1].line, None);
        let again: PdfReportSettings =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(again, s);
    }
}
