//! Words, lines and phrases (`docs/spec/pdf-reports.md` §3).

use std::fmt::Write as _;
use std::ops::Range;

use icu_normalizer::ComposingNormalizerBorrowed;

use crate::extract::Glyph;

/// The text of a report: pages of lines of words, before a template groups phrases.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Text {
    pub(crate) pages: Vec<Vec<Line>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Line {
    /// Vertical center, points from the top of the page.
    pub(crate) center: f64,
    pub(crate) words: Vec<Word>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Word {
    pub(crate) text: String,
    /// `text` lower-cased, for matching template texts.
    pub(crate) key: String,
    pub(crate) left: f64,
    pub(crate) right: f64,
    pub(crate) size: f64,
}

/// One page as a template sees it, for writing and checking templates (the test tool).
#[derive(Debug, Clone, PartialEq)]
pub struct PageLayout {
    /// Page number, from 1.
    pub number: usize,
    /// Lines top to bottom.
    pub lines: Vec<LineLayout>,
}

/// One line of a [`PageLayout`].
#[derive(Debug, Clone, PartialEq)]
pub struct LineLayout {
    /// Vertical center of the line, points from the top of the page.
    pub center: f64,
    /// Phrases left to right.
    pub phrases: Vec<PhraseLayout>,
}

/// One phrase of a [`LineLayout`].
#[derive(Debug, Clone, PartialEq)]
pub struct PhraseLayout {
    /// The phrase's words joined by single spaces.
    pub text: String,
    /// Left edge, points from the left of the page.
    pub left: f64,
    /// Right edge, points from the left of the page.
    pub right: f64,
    /// Font size of its first word, points.
    pub size: f64,
}

impl Text {
    pub(crate) fn new(pages: Vec<Vec<Line>>) -> Self {
        Self { pages }
    }

    /// Number of pages.
    pub fn pages(&self) -> usize {
        self.pages.len()
    }

    /// Number of words on all pages.
    pub fn words(&self) -> usize {
        self.pages
            .iter()
            .flatten()
            .map(|line| line.words.len())
            .sum()
    }

    /// Lines and phrases with their positions, phrases grouped by `phrase_gap`.
    pub fn layout(&self, phrase_gap: f64) -> Vec<PageLayout> {
        self.pages
            .iter()
            .enumerate()
            .map(|(i, lines)| PageLayout {
                number: i + 1,
                lines: lines
                    .iter()
                    .map(|line| LineLayout {
                        center: line.center,
                        phrases: phrases(&line.words, phrase_gap)
                            .into_iter()
                            .map(|r| PhraseLayout {
                                text: join(&line.words[r.clone()]),
                                left: line.words[r.start].left,
                                right: line.words[r.end - 1].right,
                                size: line.words[r.start].size,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect()
    }

    /// The text as the message detail shows it: `Page n` before each page, one line per text
    /// line, a tab between phrases.
    pub fn render(&self, phrase_gap: f64) -> String {
        let mut out = String::new();
        for page in self.layout(phrase_gap) {
            if page.number > 1 {
                out.push('\n');
            }
            let _ = writeln!(out, "Page {}", page.number);
            for line in page.lines {
                let phrases: Vec<&str> = line.phrases.iter().map(|p| p.text.as_str()).collect();
                let _ = writeln!(out, "{}", phrases.join("\t"));
            }
        }
        out
    }
}

/// The words of `words` joined by single spaces.
pub(crate) fn join(words: &[Word]) -> String {
    let texts: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
    texts.join(" ")
}

/// Ranges of `words` that form phrases: a gap of at least `gap` × the left word's font size
/// starts a new phrase.
pub(crate) fn phrases(words: &[Word], gap: f64) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    for i in 1..words.len() {
        if words[i].left - words[i - 1].right >= gap * words[i - 1].size {
            out.push(start..i);
            start = i;
        }
    }
    if !words.is_empty() {
        out.push(start..words.len());
    }
    out
}

/// A glyph's vertical center: the baseline minus 0.3 em (the middle of a box from 0.8 em above
/// to 0.2 em below the baseline).
fn center(glyph: &Glyph) -> f64 {
    glyph.baseline - 0.3 * glyph.size
}

/// Lines of one page, top to bottom. Glyphs whose centers lie within half of the smaller font
/// size of the line's first glyph join that line.
pub(crate) fn lines(mut glyphs: Vec<Glyph>) -> Vec<Line> {
    glyphs.sort_by(|a, b| center(a).total_cmp(&center(b)));
    let mut groups: Vec<(f64, f64, Vec<Glyph>)> = Vec::new();
    for glyph in glyphs {
        let c = center(&glyph);
        match groups.last_mut() {
            Some((anchor, size, group)) if c - *anchor <= 0.5 * size.min(glyph.size) => {
                *size = size.min(glyph.size);
                group.push(glyph);
            }
            _ => groups.push((c, glyph.size, vec![glyph])),
        }
    }
    groups
        .into_iter()
        .filter_map(|(anchor, _, glyphs)| {
            let words = words(glyphs);
            (!words.is_empty()).then_some(Line {
                center: anchor,
                words,
            })
        })
        .collect()
}

/// Words of one line, left to right: a space or a gap wider than 0.25 em ends a word. A glyph
/// drawn again at almost the same place (simulated bold) is not repeated.
fn words(mut glyphs: Vec<Glyph>) -> Vec<Word> {
    glyphs.sort_by(|a, b| a.left.total_cmp(&b.left));
    let mut words = Vec::new();
    let mut current: Option<Builder> = None;
    for glyph in glyphs {
        match current.as_mut() {
            _ if glyph.space => words.extend(current.take().map(Builder::finish)),
            Some(word) if word.repeats(&glyph) => {}
            Some(word) if glyph.left - word.right <= 0.25 * word.size => word.push(glyph),
            _ => {
                words.extend(current.take().map(Builder::finish));
                current = Some(Builder::new(glyph));
            }
        }
    }
    words.extend(current.map(Builder::finish));
    words
}

struct Builder {
    text: String,
    left: f64,
    right: f64,
    size: f64,
    last: Glyph,
}

impl Builder {
    fn new(glyph: Glyph) -> Self {
        Self {
            text: glyph.text.clone(),
            left: glyph.left,
            right: glyph.right,
            size: glyph.size,
            last: glyph,
        }
    }

    fn repeats(&self, glyph: &Glyph) -> bool {
        glyph.text == self.last.text && (glyph.left - self.last.left).abs() < 0.1 * self.size
    }

    fn push(&mut self, glyph: Glyph) {
        self.text.push_str(&glyph.text);
        self.right = self.right.max(glyph.right);
        self.last = glyph;
    }

    fn finish(self) -> Word {
        let text = ComposingNormalizerBorrowed::new_nfc()
            .normalize(&self.text)
            .into_owned();
        Word {
            key: text.to_lowercase(),
            text,
            left: self.left,
            right: self.right,
            size: self.size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(text: &str, left: f64, baseline: f64) -> Glyph {
        Glyph {
            text: text.into(),
            left,
            right: left + 5.0,
            baseline,
            size: 10.0,
            space: text == " ",
        }
    }

    fn texts(lines: &[Line]) -> Vec<Vec<&str>> {
        lines
            .iter()
            .map(|l| l.words.iter().map(|w| w.text.as_str()).collect())
            .collect()
    }

    #[test]
    fn glyphs_form_words_and_lines_in_reading_order() {
        let glyphs = vec![
            glyph("b", 105.0, 100.0),
            glyph("a", 100.0, 100.0),
            glyph(" ", 110.0, 100.0),
            glyph("c", 115.0, 101.0),
            glyph("d", 130.0, 100.0),
            glyph("e", 100.0, 120.0),
        ];
        assert_eq!(texts(&lines(glyphs)), [vec!["ab", "c", "d"], vec!["e"]]);
    }

    #[test]
    fn simulated_bold_is_read_once() {
        let glyphs = vec![
            glyph("A", 100.0, 50.0),
            glyph("A", 100.3, 50.0),
            glyph("B", 105.0, 50.0),
        ];
        assert_eq!(texts(&lines(glyphs)), [vec!["AB"]]);
    }

    #[test]
    fn decomposed_text_is_normalized() {
        let glyphs = vec![glyph("e\u{301}", 100.0, 50.0)];
        assert_eq!(lines(glyphs)[0].words[0].text, "é");
    }

    #[test]
    fn phrases_split_at_wide_gaps() {
        let lines = lines(vec![
            glyph("a", 0.0, 10.0),
            glyph("b", 15.0, 10.0),
            glyph("c", 50.0, 10.0),
        ]);
        assert_eq!(phrases(&lines[0].words, 2.0), [0..2, 2..3]);
        let text = Text::new(vec![lines]);
        assert_eq!(text.render(2.0), "Page 1\na b\tc\n");
    }
}
