//! Positioned text from a PDF file (`docs/spec/pdf-reports.md` §3).
//!
//! lopdf reads the file structure (cross-reference recovery, decryption with the empty
//! password, bounded decompression, content tokens). The engine walks the content itself:
//! text operators, the graphics state and form XObjects, placing every glyph on the displayed
//! page (crop box and rotation applied, origin top left).

mod font;
mod interp;
mod matrix;
mod standard14;

use std::time::Instant;

use lopdf::{DecompressError, Document, LoadOptions, Object, Stream};

use crate::layout::{self, Text};
use crate::{Error, Limits, Reason};

/// One shown glyph in page coordinates (points, origin top left, y downwards).
#[derive(Debug, Clone)]
pub(crate) struct Glyph {
    pub(crate) text: String,
    pub(crate) left: f64,
    pub(crate) right: f64,
    pub(crate) baseline: f64,
    pub(crate) size: f64,
    /// A space (or no-break space): separates words, carries no text.
    pub(crate) space: bool,
}

/// Glyphs allowed per word on average before the word limit is assumed to be exceeded.
const GLYPHS_PER_WORD: usize = 16;

/// What one file may still consume: decompressed bytes, glyphs and time.
pub(crate) struct Budget {
    bytes: usize,
    glyphs: usize,
    deadline: Option<Instant>,
    ticks: u32,
}

impl Budget {
    fn new(limits: &Limits) -> Self {
        Self {
            bytes: limits.max_content_bytes,
            glyphs: limits.max_words.saturating_mul(GLYPHS_PER_WORD),
            deadline: limits.deadline,
            ticks: 0,
        }
    }

    pub(crate) fn remaining(&self) -> usize {
        self.bytes
    }

    pub(crate) fn exceeded() -> Error {
        Error::new(
            Reason::Limits,
            "the decompressed content exceeds the file's limit",
        )
    }

    fn take(&mut self, bytes: usize) -> Result<(), Error> {
        self.bytes = self.bytes.checked_sub(bytes).ok_or_else(Self::exceeded)?;
        Ok(())
    }

    /// A stream's decoded bytes within the budget; `None` when it cannot be decoded.
    pub(crate) fn stream(&mut self, stream: &Stream) -> Result<Option<Vec<u8>>, Error> {
        match stream.get_plain_content_with_limit(self.bytes) {
            Ok(bytes) => {
                self.take(bytes.len())?;
                Ok(Some(bytes))
            }
            Err(error) if is_limit(&error) => Err(Self::exceeded()),
            Err(_) => Ok(None),
        }
    }

    /// Checked once per shown glyph, which also counts toward the time check: one string
    /// operand can show many glyphs.
    pub(crate) fn glyph(&mut self) -> Result<(), Error> {
        self.glyphs = self.glyphs.checked_sub(1).ok_or_else(|| {
            Error::new(Reason::Limits, "the file has more text than its word limit")
        })?;
        self.tick()
    }

    /// Checked once per content operation and glyph; the clock is read every 256 of them.
    pub(crate) fn tick(&mut self) -> Result<(), Error> {
        self.ticks = self.ticks.wrapping_add(1);
        if self.ticks.is_multiple_of(256) && self.deadline.is_some_and(|d| Instant::now() > d) {
            return Err(Error::new(
                Reason::Limits,
                "reading the file took longer than its time limit",
            ));
        }
        Ok(())
    }
}

fn is_limit(error: &lopdf::Error) -> bool {
    matches!(
        error,
        lopdf::Error::Decompress(DecompressError::MemoryLimitExceeded { .. })
    )
}

/// Extracts the text of every page.
pub(crate) fn extract(bytes: &[u8], limits: &Limits) -> Result<Text, Error> {
    let doc = load(bytes, limits)?;
    let pages = doc.get_pages();
    check_pages(&doc, pages.len(), limits)?;
    let mut budget = Budget::new(limits);
    let mut text = Vec::with_capacity(pages.len());
    for (number, id) in pages {
        let glyphs = interp::page(&doc, id, number, &mut budget)?;
        text.push(layout::lines(glyphs));
    }
    let text = Text::new(text);
    if text.words() > limits.max_words {
        return Err(Error::new(
            Reason::Limits,
            "the file has more words than its limit",
        ));
    }
    if text.words() == 0 {
        return Err(Error::new(Reason::NoText, "no page has text"));
    }
    Ok(text)
}

fn load(bytes: &[u8], limits: &Limits) -> Result<Document, Error> {
    let options = LoadOptions {
        max_decompressed_size: Some(limits.max_content_bytes),
        ..LoadOptions::default()
    };
    let doc = Document::load_mem_with_options(bytes, options).map_err(|error| {
        if is_limit(&error) {
            Budget::exceeded()
        } else {
            Error::unreadable("the file is not a readable PDF")
        }
    })?;
    if doc.is_encrypted() {
        return Err(Error::unreadable("the file needs a password to open"));
    }
    Ok(doc)
}

/// The page tree must yield every page it declares, and no more than the limit.
fn check_pages(doc: &Document, found: usize, limits: &Limits) -> Result<(), Error> {
    let declared = doc
        .catalog()
        .ok()
        .and_then(|c| c.get(b"Pages").ok())
        .and_then(|o| match o {
            Object::Reference(id) => doc.get_dictionary(*id).ok(),
            Object::Dictionary(d) => Some(d),
            _ => None,
        })
        .and_then(|pages| pages.get(b"Count").ok())
        .and_then(|count| count.as_i64().ok())
        .and_then(|count| usize::try_from(count).ok());
    if found == 0 {
        return Err(Error::unreadable("the file has no readable page"));
    }
    if let Some(declared) = declared.filter(|d| found < *d) {
        return Err(Error::unreadable(format!(
            "the file declares {declared} pages; {found} can be read"
        )));
    }
    let pages = declared.unwrap_or(found).max(found);
    if pages > limits.max_pages {
        return Err(Error::new(
            Reason::Limits,
            format!(
                "the file has {pages} pages; at most {} are read",
                limits.max_pages
            ),
        ));
    }
    Ok(())
}
