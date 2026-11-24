//! The PDF report test tool (`docs/spec/pdf-reports.md` §9): reads a report with a template,
//! and maps it as a destination would, without storing or sending anything.

use packie_core::MessageId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::messages::FieldValue;

/// Largest file the test accepts, decoded.
pub const MAX_TEST_FILE_BYTES: usize = 8 * 1024 * 1024;

/// A value of the encoded destination message longer than this is shortened.
pub const SHORTEN_ABOVE: usize = 1000;

/// How much of a shortened value is kept.
pub const SHORTENED_TO: usize = 100;

/// What to read and with which template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PdfReportTest {
    /// A `pdf_report` template, written as in the runtime document.
    #[ts(type = "Record<string, unknown>")]
    pub settings: serde_json::Value,
    /// The report, Base64-encoded; at most [`MAX_TEST_FILE_BYTES`] decoded.
    #[serde(default)]
    pub file: Option<String>,
    /// Or a stored `pdf_report` message, whose original is read.
    #[serde(default)]
    pub message_id: Option<MessageId>,
    /// A destination to map the report for.
    #[serde(default)]
    pub destination: Option<TestDestination>,
}

/// A destination written as in the runtime document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct TestDestination {
    /// The destination protocol (`hl7_mllp`, `delimited_file`, ...).
    pub protocol: String,
    /// Its settings.
    #[ts(type = "Record<string, unknown>")]
    pub settings: serde_json::Value,
    /// Its mapping; required unless the protocol is the source's.
    #[serde(default)]
    #[ts(type = "Record<string, unknown> | null")]
    pub mapping: Option<serde_json::Value>,
}

/// What the test found. Report content: the console conceals it like a message body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PdfReportTestResult {
    /// The report reads with the template.
    pub ok: bool,
    /// Why not: `pdf_unreadable`, `pdf_no_text`, `pdf_limits`, `template_mismatch`,
    /// `field_invalid`.
    pub reason_code: Option<String>,
    /// What failed, in template terms.
    pub reason_detail: Option<String>,
    /// The extracted lines with their phrases and positions, to write or correct a template;
    /// empty when the file has no readable text.
    pub pages: Vec<PdfPage>,
    /// The model filters and mappings read (`field.<name>`, `report.pages`), without the
    /// embedded file; empty when the report does not read.
    pub fields: Vec<FieldValue>,
    /// The encoded destination message, values over [`SHORTEN_ABOVE`] characters shortened;
    /// when a destination was given and the report reads.
    pub output: Option<String>,
    /// The mapping's transform error instead of `output`.
    pub output_error: Option<String>,
}

/// One page of extracted text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PdfPage {
    /// From 1.
    pub number: u32,
    /// Top to bottom.
    pub lines: Vec<PdfLine>,
}

/// One line of a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PdfLine {
    /// Vertical center, points from the top of the page.
    pub center: f64,
    /// Left to right.
    pub phrases: Vec<PdfPhrase>,
}

/// One phrase of a line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PdfPhrase {
    /// The phrase's words joined by single spaces.
    pub text: String,
    /// Left edge, points from the left of the page.
    pub left: f64,
    /// Right edge, points.
    pub right: f64,
    /// Font size, points.
    pub size: f64,
}
