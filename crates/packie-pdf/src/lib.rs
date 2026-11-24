//! Pure reader of born-digital PDF reports (`docs/spec/pdf-reports.md`): positioned text and
//! the labelled fields a [`Template`] locates. No I/O.
//!
//! [`extract()`] reads the text layer of a file into [`Text`]: words on lines on pages, in
//! points from the top left of each displayed page. [`Template::read`] finds the template's
//! labels and reads their values into a [`Report`]. Both fail with an [`Error`] whose
//! [`Reason`] is the message's reason code and whose detail names template items and
//! positions, never text from the report.
//!
//! The template mirrors the connector settings of `docs/spec/configuration.md` §7.7 except
//! the field types and formats, which the caller checks (they share the mapping date formats).
//!
//! ```
//! use packie_pdf::{Field, Limits, Position, Template};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let pdf: &[u8] = b"%PDF-1.4
//! # 1 0 obj <</Type/Catalog/Pages 2 0 R>> endobj
//! # 2 0 obj <</Type/Pages/Kids[3 0 R]/Count 1>> endobj
//! # 3 0 obj <</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R
//! #   /Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>> endobj
//! # 4 0 obj <</Length 52>> stream
//! # BT /F1 10 Tf 72 700 Td (Lab Number: SYN-0001) Tj ET
//! # endstream endobj
//! # trailer <</Root 1 0 R>>
//! # %%EOF";
//! let text = packie_pdf::extract(pdf, &Limits::default())?; // the bytes of a PDF file
//! let template = Template {
//!     fields: vec![Field {
//!         name: "sample_id".into(),
//!         labels: vec!["Lab Number:".into()],
//!         position: Position::Right,
//!         occurrence: 0,
//!         required: true,
//!     }],
//!     ..Template::default()
//! };
//! let report = template.read(&text)?;
//! assert_eq!(report.field("sample_id"), Some("SYN-0001"));
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![allow(
    clippy::float_arithmetic,
    reason = "page geometry: positions and sizes in points; field values are text, never floats"
)]

mod error;
mod extract;
mod layout;
mod template;

use std::time::Instant;

pub use error::{Error, Reason};
pub use layout::{LineLayout, PageLayout, PhraseLayout, Text};
pub use template::{Field, Position, Report, Template};

/// Limits of one file (`docs/spec/pdf-reports.md` §8). The file size limit is the caller's.
#[derive(Debug, Clone)]
pub struct Limits {
    /// Pages a file may have (declared or found).
    pub max_pages: usize,
    /// Decompressed bytes across all streams the reader decodes.
    pub max_content_bytes: usize,
    /// Words across all pages.
    pub max_words: usize,
    /// When reading must have finished; `None` for no time limit.
    pub deadline: Option<Instant>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_pages: 20,
            max_content_bytes: 64 << 20,
            max_words: 200_000,
            deadline: None,
        }
    }
}

/// Reads the text layer of a PDF file.
pub fn extract(bytes: &[u8], limits: &Limits) -> Result<Text, Error> {
    extract::extract(bytes, limits)
}
