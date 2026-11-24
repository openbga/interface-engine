//! PDF report adapter (read-only): the fields a template read, and the report itself.
//!
//! | Path | Content |
//! | --- | --- |
//! | `field.<name>` | a template field's value; empty when the report has none |
//! | `report.pages` | the number of pages |
//! | `report.file_base64` | the original file in Base64 (standard alphabet, padded, no line breaks), encoded when first read |
//!
//! [`PdfModel`] stands on its own like [`crate::RecordModel`]: the PDF reader (`packie-pdf`)
//! hands its fields over as `(name, value)` pairs, so this crate does not depend on it.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::OnceLock;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::error::ModelError;
use crate::model::Model;
use crate::path::{FieldRef, Path, Syntax};

/// The names under `report.`.
pub(crate) const REPORT_FIELDS: [&str; 2] = ["pages", "file_base64"];

/// A PDF report read through [`Model`].
#[derive(Clone, PartialEq, Eq)]
pub struct PdfModel {
    fields: BTreeMap<String, String>,
    pages: String,
    file: Vec<u8>,
    encoded: OnceLock<String>,
}

impl fmt::Debug for PdfModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PdfModel")
            .field("fields", &self.fields)
            .field("pages", &self.pages)
            .field("file_bytes", &self.file.len())
            .finish_non_exhaustive()
    }
}

impl PdfModel {
    /// A report with its template fields, page count and original file.
    #[must_use]
    pub fn new(
        fields: impl IntoIterator<Item = (String, String)>,
        pages: usize,
        file: Vec<u8>,
    ) -> Self {
        Self {
            fields: fields.into_iter().collect(),
            pages: pages.to_string(),
            file,
            encoded: OnceLock::new(),
        }
    }

    /// The template fields and their values, in name order.
    pub fn fields(&self) -> impl Iterator<Item = (&str, &str)> {
        self.fields.iter().map(|(n, v)| (n.as_str(), v.as_str()))
    }
}

impl Model for PdfModel {
    fn syntax(&self) -> Syntax {
        Syntax::Pdf
    }

    fn get(&self, path: &Path) -> Option<Cow<'_, str>> {
        let FieldRef::Name(name) = path.field() else {
            return None;
        };
        if !path.is_absolute_in(Syntax::Pdf) {
            return None;
        }
        let text = match (path.record()?, name.as_str()) {
            ("field", name) => self.fields.get(name)?.as_str(),
            ("report", "pages") => self.pages.as_str(),
            ("report", "file_base64") => self.encoded.get_or_init(|| STANDARD.encode(&self.file)),
            _ => return None,
        };
        Some(Cow::Borrowed(text))
    }

    fn set(&mut self, _path: &Path, _value: &str) -> Result<(), ModelError> {
        Err(ModelError::ReadOnly)
    }

    fn count(&self, _record: &str) -> usize {
        0
    }

    fn create(&mut self, _record: &str) -> Result<usize, ModelError> {
        Err(ModelError::ReadOnly)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str) -> Path {
        Path::parse(Syntax::Pdf, text).expect("valid path")
    }

    fn report() -> PdfModel {
        let fields = [("sample_id", "SYN-0001"), ("medical_record", "")];
        PdfModel::new(
            fields.map(|(n, v)| (n.to_owned(), v.to_owned())),
            2,
            b"%PDF-1.7 synthetic".to_vec(),
        )
    }

    #[test]
    fn reads_fields_and_the_report() {
        let m = report();
        assert_eq!(m.get(&p("field.sample_id")).as_deref(), Some("SYN-0001"));
        assert_eq!(m.get(&p("field.medical_record")).as_deref(), Some(""));
        assert_eq!(m.get(&p("field.unknown")), None);
        assert_eq!(m.get(&p("report.pages")).as_deref(), Some("2"));
        assert_eq!(
            m.get(&p("report.file_base64")).as_deref(),
            Some("JVBERi0xLjcgc3ludGhldGlj")
        );
        assert_eq!(m.get(&p("report.size")), None);
        assert_eq!(m.count("field"), 0);
    }

    #[test]
    fn the_model_is_read_only_and_debug_omits_the_file() {
        let mut m = report();
        assert_eq!(m.set(&p("field.sample_id"), "x"), Err(ModelError::ReadOnly));
        assert_eq!(m.create("field"), Err(ModelError::ReadOnly));
        assert!(!format!("{m:?}").contains("PDF-1.7"));
    }
}
