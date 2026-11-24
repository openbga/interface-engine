//! Why a report cannot be read (`docs/spec/pdf-reports.md` §7).

/// The reason code of a report that cannot be read. `field_invalid` belongs to the caller,
/// which checks field types against the template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Not a PDF, damaged beyond recovery, needs a password, or uses an unsupported feature.
    Unreadable,
    /// No text on any page, or a glyph whose character cannot be determined.
    NoText,
    /// A limit (pages, decompressed content, words, time) was reached.
    Limits,
    /// The report does not match its template.
    TemplateMismatch,
}

impl Reason {
    /// The reason code stored with an `invalid` message.
    pub fn code(self) -> &'static str {
        match self {
            Self::Unreadable => "pdf_unreadable",
            Self::NoText => "pdf_no_text",
            Self::Limits => "pdf_limits",
            Self::TemplateMismatch => "template_mismatch",
        }
    }
}

/// A reason with a detail in terms of the template and positions only: never text read from
/// the report, so it can be logged and shown.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}: {detail}", reason.code())]
pub struct Error {
    /// The reason code.
    pub reason: Reason,
    /// What failed, for `reason_detail`.
    pub detail: String,
}

impl Error {
    pub(crate) fn new(reason: Reason, detail: impl Into<String>) -> Self {
        Self {
            reason,
            detail: detail.into(),
        }
    }

    pub(crate) fn unreadable(detail: impl Into<String>) -> Self {
        Self::new(Reason::Unreadable, detail)
    }

    pub(crate) fn mismatch(detail: impl Into<String>) -> Self {
        Self::new(Reason::TemplateMismatch, detail)
    }
}
