//! Reading PDF reports with a connector's template (`docs/spec/pdf-reports.md`): the
//! message pipeline, the test tool and the message views use one reader.

use std::time::{Duration, Instant};

use packie_config::{ColumnType, FieldPosition, OneOrMore, PdfField, PdfReportSettings};
use packie_transform::ValueType;
use rust_decimal::prelude::ToPrimitive;

/// Extraction and template work allowed per report (`pdf-reports.md` §8).
const READ_TIME: Duration = Duration::from_secs(10);

/// Why a report does not parse: the reason code and what failed, in template terms (never
/// report text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    /// `pdf_unreadable`, `pdf_no_text`, `pdf_limits`, `template_mismatch`, `field_invalid`.
    pub code: String,
    /// The `reason_detail`.
    pub detail: String,
}

impl From<packie_pdf::Error> for ReadError {
    fn from(error: packie_pdf::Error) -> Self {
        Self {
            code: error.reason.code().to_owned(),
            detail: error.detail,
        }
    }
}

/// A connector's template, compiled for reading reports.
#[derive(Debug)]
pub struct PdfReader {
    template: packie_pdf::Template,
    types: Vec<(String, ValueType, ColumnType)>,
    max_pages: usize,
}

impl PdfReader {
    /// Compiles a connector's settings. Validation compiled every field type; one that does
    /// not compile reads as text.
    #[must_use]
    pub fn new(settings: &PdfReportSettings) -> Self {
        let types = settings
            .fields
            .iter()
            .filter(|f| f.field_type != ColumnType::String)
            .filter_map(|f| {
                let formats = f.format.as_ref().map(OneOrMore::to_vec).unwrap_or_default();
                let kind = ValueType::compile(f.field_type.as_str(), &formats).ok()?;
                Some((f.name.clone(), kind, f.field_type))
            })
            .collect();
        Self {
            template: packie_pdf::Template {
                phrase_gap: settings.phrase_gap.to_f64().unwrap_or(2.0),
                label_area_end: settings.label_area_end.clone(),
                fields: settings.fields.iter().map(field).collect(),
            },
            types,
            max_pages: usize::try_from(settings.max_pages).unwrap_or(usize::MAX),
        }
    }

    /// The extracted text of a report: a `Page n` line before each page, one line per text
    /// line, a tab between phrases.
    ///
    /// # Errors
    /// The reason the file cannot be read.
    pub fn text(&self, original: &[u8]) -> Result<String, ReadError> {
        Ok(self.extract(original)?.render(self.phrase_gap()))
    }

    /// The phrase gap of the template, for laying out extracted text.
    #[must_use]
    pub const fn phrase_gap(&self) -> f64 {
        self.template.phrase_gap
    }

    /// The text layer of a report, within the limits of `pdf-reports.md` §8.
    ///
    /// # Errors
    /// The reason the file cannot be read.
    pub fn extract(&self, original: &[u8]) -> Result<packie_pdf::Text, ReadError> {
        let limits = packie_pdf::Limits {
            max_pages: self.max_pages,
            deadline: Instant::now().checked_add(READ_TIME),
            ..packie_pdf::Limits::default()
        };
        Ok(packie_pdf::extract(original, &limits)?)
    }

    /// The template's fields from extracted text, their types checked.
    ///
    /// # Errors
    /// `template_mismatch` or `field_invalid` with what failed.
    pub fn fields(&self, text: &packie_pdf::Text) -> Result<packie_pdf::Report, ReadError> {
        let report = self.template.read(text)?;
        for (name, value) in &report.fields {
            let Some((_, kind, column)) = self.types.iter().find(|(n, ..)| n == name) else {
                continue;
            };
            if !kind.accepts(value) {
                let what = match column {
                    ColumnType::Date | ColumnType::Datetime => "formats",
                    _ => "type",
                };
                return Err(ReadError {
                    code: "field_invalid".to_owned(),
                    detail: format!("field \"{name}\" does not match its {what}"),
                });
            }
        }
        Ok(report)
    }

    /// Extraction and fields together.
    ///
    /// # Errors
    /// As [`extract`](Self::extract) and [`fields`](Self::fields).
    pub fn read(&self, original: &[u8]) -> Result<packie_pdf::Report, ReadError> {
        self.fields(&self.extract(original)?)
    }
}

impl Default for PdfReader {
    /// A reader without fields, with the default phrase gap and limits: it extracts text
    /// only.
    fn default() -> Self {
        Self {
            template: packie_pdf::Template::default(),
            types: Vec::new(),
            max_pages: packie_pdf::Limits::default().max_pages,
        }
    }
}

fn field(field: &PdfField) -> packie_pdf::Field {
    let position = match field.position {
        FieldPosition::Right => packie_pdf::Position::Right,
        FieldPosition::Below => packie_pdf::Position::Below {
            line: field.line.map_or(1, |n| usize::try_from(n).unwrap_or(1)),
            phrase: field.phrase.unwrap_or(0),
            indent: field.indent.and_then(|d| d.to_f64()).unwrap_or(4.0),
        },
    };
    packie_pdf::Field {
        name: field.name.clone(),
        labels: field.label.to_vec(),
        position,
        occurrence: field.occurrence,
        required: field.required,
    }
}
