//! Labelled fields located on the page (`docs/spec/pdf-reports.md` §4).

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use icu_normalizer::ComposingNormalizerBorrowed;

use crate::Error;
use crate::layout::{self, Line, Text, Word};

/// Where a field's value sits relative to its label.
#[derive(Debug, Clone, PartialEq)]
pub enum Position {
    /// On the label's line: the rest of its phrase, or the next phrase.
    Right,
    /// In the block of lines under the label.
    Below {
        /// Block line, from 1.
        line: usize,
        /// Phrase of that line: 0 the first, −1 the last.
        phrase: i32,
        /// How far right of the label's left edge a block line may start, points.
        indent: f64,
    },
}

/// A labelled field of the template.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Model name (`field.<name>`).
    pub name: String,
    /// The label, or alternative labels; the first found on a page is used.
    pub labels: Vec<String>,
    /// Where the value sits.
    pub position: Position,
    /// Which match of the label on a page: 0 the first, 1 the second, −1 the last.
    pub occurrence: i32,
    /// Read on page 1 only; the label on later pages (a footer repeating it) is ignored.
    pub first_page_only: bool,
    /// The label must be on page 1 with a value.
    pub required: bool,
}

/// The settings that locate fields (`docs/spec/configuration.md` §7.7).
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    /// A gap of at least this many times the font size separates phrases.
    pub phrase_gap: f64,
    /// Texts whose line ends the label area.
    pub label_area_end: Vec<String>,
    /// The fields to read.
    pub fields: Vec<Field>,
}

impl Default for Template {
    fn default() -> Self {
        Self {
            phrase_gap: 2.0,
            label_area_end: Vec::new(),
            fields: Vec::new(),
        }
    }
}

/// The values a template read from a report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Number of pages.
    pub pages: usize,
    /// Every template field in template order; empty when the report has no value.
    pub fields: Vec<(String, String)>,
}

impl Report {
    /// The value of a field, `None` for a name the template does not have.
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Tolerance left of a label's edge for values below it, points.
const LEFT_TOLERANCE: f64 = 4.0;

/// One accepted label match: field, alternative, line and word range on a page.
#[derive(Debug, Clone, Copy)]
struct Hit {
    field: usize,
    alt: usize,
    line: usize,
    start: usize,
    len: usize,
}

struct Page<'t> {
    lines: Vec<PageLine<'t>>,
}

struct PageLine<'t> {
    center: f64,
    words: &'t [Word],
    phrases: Vec<Range<usize>>,
}

/// Accepted matches of one page and the words they cover.
struct Hits {
    list: Vec<Hit>,
    starts: HashSet<(usize, usize)>,
    covered: HashSet<(usize, usize)>,
}

impl Template {
    /// Reads the fields from extracted text.
    pub fn read(&self, text: &Text) -> Result<Report, Error> {
        let labels: Vec<Vec<Vec<String>>> = self
            .fields
            .iter()
            .map(|f| f.labels.iter().map(|l| keys(l)).collect())
            .collect();
        let ends: Vec<Vec<String>> = self.label_area_end.iter().map(|t| keys(t)).collect();
        let mut found: Vec<Vec<(usize, String)>> = vec![Vec::new(); self.fields.len()];
        for (number, lines) in text.pages.iter().enumerate() {
            let page = Page::new(lines, self.phrase_gap);
            let hits = page.hits(&labels);
            if number == 0 {
                page.check_label_area(&hits, &ends)?;
            }
            let fields = page.values(&hits, &self.fields).into_iter();
            for (index, value) in
                fields.filter(|(i, _)| number == 0 || !self.fields[*i].first_page_only)
            {
                found[index].push((number, value));
            }
        }
        let fields = self
            .fields
            .iter()
            .zip(&found)
            .map(|(field, found)| Ok((field.name.clone(), resolve(field, found)?)))
            .collect::<Result<_, Error>>()?;
        Ok(Report {
            pages: text.pages(),
            fields,
        })
    }
}

/// A template text as lower-cased words, NFC-normalized like the page's text.
fn keys(text: &str) -> Vec<String> {
    ComposingNormalizerBorrowed::new_nfc()
        .normalize(text)
        .split_whitespace()
        .map(str::to_lowercase)
        .collect()
}

/// The match of a field on a page: the first alternative found, then `occurrence`.
fn pick(hits: &[Hit], field: usize, occurrence: i32) -> Option<Hit> {
    let alt = hits
        .iter()
        .filter(|h| h.field == field)
        .map(|h| h.alt)
        .min()?;
    let mut matches = hits.iter().filter(|h| h.field == field && h.alt == alt);
    match usize::try_from(occurrence) {
        Ok(n) => matches.nth(n).copied(),
        Err(_) => matches.next_back().copied(),
    }
}

/// Required fields need their label and a value on page 1; values found on several pages
/// must agree.
fn resolve(field: &Field, found: &[(usize, String)]) -> Result<String, Error> {
    if field.required {
        match found.first() {
            Some((0, value)) if !value.is_empty() => {}
            Some((0, _)) => {
                return Err(Error::mismatch(format!(
                    "field \"{}\" is empty on page 1",
                    field.name
                )));
            }
            _ => {
                let label = field.labels.first().map_or("", String::as_str);
                return Err(Error::mismatch(format!(
                    "label \"{label}\" not found on page 1"
                )));
            }
        }
    }
    let mut values = found.iter().filter(|(_, v)| !v.is_empty());
    let Some((first_page, first)) = values.next() else {
        return Ok(String::new());
    };
    if let Some((page, _)) = values.find(|(_, v)| v != first) {
        return Err(Error::mismatch(format!(
            "field \"{}\" has different values on pages {} and {}",
            field.name,
            first_page + 1,
            page + 1
        )));
    }
    Ok(first.clone())
}

impl<'t> Page<'t> {
    fn new(lines: &'t [Line], phrase_gap: f64) -> Self {
        let lines = lines
            .iter()
            .map(|l| PageLine {
                center: l.center,
                words: &l.words,
                phrases: layout::phrases(&l.words, phrase_gap),
            })
            .collect();
        Self { lines }
    }

    /// Every label match on the page; where matches overlap, the longest wins.
    fn hits(&self, labels: &[Vec<Vec<String>>]) -> Hits {
        let mut index: HashMap<&str, Vec<(usize, usize)>> = HashMap::new();
        for (l, line) in self.lines.iter().enumerate() {
            for (w, word) in line.words.iter().enumerate() {
                index.entry(word.key.as_str()).or_default().push((l, w));
            }
        }
        let mut candidates: Vec<Hit> = labels
            .iter()
            .enumerate()
            .flat_map(|(field, alts)| {
                alts.iter()
                    .enumerate()
                    .map(move |(alt, label)| (field, alt, label))
            })
            .filter_map(|(field, alt, label)| {
                Some((field, alt, label, index.get(label.first()?.as_str())?))
            })
            .flat_map(|(field, alt, label, positions)| {
                positions
                    .iter()
                    .filter(|(line, start)| self.matches(*line, *start, label))
                    .map(move |&(line, start)| Hit {
                        field,
                        alt,
                        line,
                        start,
                        len: label.len(),
                    })
            })
            .collect();
        candidates.sort_by_key(|h| (std::cmp::Reverse(h.len), h.line, h.start, h.field, h.alt));
        let mut covered = HashSet::new();
        let mut list = Vec::new();
        for hit in candidates {
            let words: Vec<(usize, usize)> = (hit.start..hit.start + hit.len)
                .map(|w| (hit.line, w))
                .collect();
            // Free words, or exactly the words of an accepted match (two fields may declare
            // the same label with different occurrences).
            let free = words.iter().all(|w| !covered.contains(w));
            let same = list
                .iter()
                .any(|h: &Hit| (h.line, h.start, h.len) == (hit.line, hit.start, hit.len));
            if free || same {
                covered.extend(words);
                list.push(hit);
            }
        }
        list.sort_by_key(|h| (h.line, h.start));
        let starts = list.iter().map(|h| (h.line, h.start)).collect();
        Hits {
            list,
            starts,
            covered,
        }
    }

    /// The label's words at `start`, all within one phrase.
    fn matches(&self, line: usize, start: usize, label: &[String]) -> bool {
        let line = &self.lines[line];
        let Some(phrase) = line.phrases.iter().find(|r| r.contains(&start)) else {
            return false;
        };
        start + label.len() <= phrase.end
            && line.words[start..start + label.len()]
                .iter()
                .zip(label)
                .all(|(w, l)| w.key == *l)
    }

    /// In the label area of page 1, every word ending with `:` must belong to a declared label.
    fn check_label_area(&self, hits: &Hits, ends: &[Vec<String>]) -> Result<(), Error> {
        let Some(first) = hits.list.iter().map(|h| h.line).min() else {
            return Ok(());
        };
        for (l, line) in self.lines.iter().enumerate().skip(first) {
            if l > first && ends.iter().any(|end| line.begins_with(end)) {
                break;
            }
            let unknown = line
                .words
                .iter()
                .enumerate()
                .any(|(w, word)| word.text.ends_with(':') && !hits.covered.contains(&(l, w)));
            if unknown {
                return Err(Error::mismatch(format!(
                    "unknown label in the label area on page 1, line {}",
                    l + 1
                )));
            }
        }
        Ok(())
    }

    /// The value of every field whose label the page carries, by field index.
    fn values(&self, hits: &Hits, fields: &[Field]) -> Vec<(usize, String)> {
        fields
            .iter()
            .enumerate()
            .filter_map(|(index, field)| {
                let hit = pick(&hits.list, index, field.occurrence)?;
                Some((index, self.value(hit, &field.position, &hits.starts)))
            })
            .collect()
    }

    fn value(&self, hit: Hit, position: &Position, starts: &HashSet<(usize, usize)>) -> String {
        match position {
            Position::Right => self.right(hit, starts),
            Position::Below {
                line,
                phrase,
                indent,
            } => self.below(hit, *line, *phrase, *indent, starts),
        }
    }

    /// The rest of the label's phrase, or the next phrase on the line; it stops at a label.
    fn right(&self, hit: Hit, starts: &HashSet<(usize, usize)>) -> String {
        let line = &self.lines[hit.line];
        let Some(p) = line.phrases.iter().position(|r| r.contains(&hit.start)) else {
            return String::new();
        };
        let after = hit.start + hit.len;
        let range = if after < line.phrases[p].end {
            after..line.phrases[p].end
        } else {
            match line.phrases.get(p + 1) {
                Some(next) => next.clone(),
                None => return String::new(),
            }
        };
        let end = range
            .clone()
            .find(|w| starts.contains(&(hit.line, *w)))
            .unwrap_or(range.end);
        layout::join(&line.words[range.start..end])
    }

    /// Phrase `phrase` of block line `line` under the label (spec §4.2, `below`).
    fn below(
        &self,
        hit: Hit,
        line: usize,
        phrase: i32,
        indent: f64,
        starts: &HashSet<(usize, usize)>,
    ) -> String {
        let label = &self.lines[hit.line].words[hit.start];
        let mut previous = self.lines[hit.line].center;
        let mut block: Vec<(usize, Vec<Range<usize>>)> = Vec::new();
        for (l, candidate) in self.lines.iter().enumerate().skip(hit.line + 1) {
            let limit = if block.is_empty() { 3.0 } else { 2.0 } * label.size;
            if candidate.center - previous > limit || block.len() >= line {
                break;
            }
            let counted: Vec<Range<usize>> = candidate
                .phrases
                .iter()
                .filter(|r| candidate.words[r.start].left >= label.left - LEFT_TOLERANCE)
                .cloned()
                .collect();
            let Some(first) = counted.first() else {
                continue;
            };
            if candidate.words[first.start].left > label.left + indent {
                continue;
            }
            if starts.contains(&(l, first.start)) {
                break;
            }
            previous = candidate.center;
            block.push((l, counted));
        }
        let Some((l, counted)) = line.checked_sub(1).and_then(|i| block.get(i)) else {
            return String::new();
        };
        let index = usize::try_from(phrase)
            .ok()
            .or(counted.len().checked_sub(1));
        index
            .and_then(|i| counted.get(i))
            .map(|r| layout::join(&self.lines[*l].words[r.clone()]))
            .unwrap_or_default()
    }
}

impl PageLine<'_> {
    /// The first phrase begins with `text`'s words.
    fn begins_with(&self, text: &[String]) -> bool {
        let Some(first) = self.phrases.first() else {
            return false;
        };
        !text.is_empty()
            && first.len() >= text.len()
            && self.words[first.start..first.start + text.len()]
                .iter()
                .zip(text)
                .all(|(w, t)| w.key == *t)
    }
}
