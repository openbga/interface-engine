//! Fonts: the character and advance width of each code that a text operator shows.
//!
//! Simple fonts (`Type1`, `TrueType`, `Type3`, `MMType1`) use one byte per code; composite
//! fonts (`Type0`) are read with the `Identity-H` encoding only, two bytes per code. The
//! character comes from the `/ToUnicode` map, else (simple fonts) from `/Encoding` through
//! lopdf's encoding tables. Widths come from `/Widths` or `/W`, else for the 14 standard fonts
//! from their metrics ([`super::standard14`]).

use std::collections::HashMap;

use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object};

use super::matrix::number;
use super::{Budget, standard14};
use crate::Error;

/// One code of a shown string.
pub(crate) struct Shown<'f> {
    /// The character(s); `None` when the code has no Unicode mapping.
    pub(crate) text: Option<&'f str>,
    /// Advance in text space units for a font size of 1.
    pub(crate) width: f64,
    /// A single-byte code 32, to which word spacing applies.
    pub(crate) word_space: bool,
}

pub(crate) struct Font {
    composite: bool,
    to_unicode: HashMap<u32, String>,
    /// Simple fonts: the text of each byte through `/Encoding`.
    encoding: Vec<Option<String>>,
    widths: Widths,
}

enum Widths {
    Simple {
        first: i64,
        widths: Vec<f64>,
        missing: f64,
        standard: Option<Standard>,
        scale: f64,
    },
    Composite {
        default: f64,
        /// `c [w1 w2 …]` entries, one per code.
        codes: HashMap<u32, f64>,
        /// `c_first c_last w` entries.
        ranges: Vec<(u32, u32, f64)>,
    },
}

/// Metrics of a standard font: a width table (keyed by character, or by code for the symbol
/// fonts) or a fixed pitch.
#[derive(Clone, Copy)]
enum Standard {
    Table(&'static [(u16, u16)], bool),
    Fixed(f64),
}

/// Width assumed for a code no width source covers (half an em).
const UNKNOWN_WIDTH: f64 = 500.0;

impl Font {
    pub(crate) fn load(
        doc: &Document,
        dict: &Dictionary,
        budget: &mut Budget,
    ) -> Result<Self, Error> {
        let composite = name(dict, b"Subtype") == Some(b"Type0".as_slice());
        let to_unicode = to_unicode(doc, dict, budget)?;
        if composite {
            match name(dict, b"Encoding") {
                Some(b"Identity-H") => {}
                _ => {
                    return Err(Error::unreadable(
                        "a font uses a character encoding the reader does not support",
                    ));
                }
            }
            let widths = composite_widths(doc, dict);
            return Ok(Self {
                composite,
                to_unicode,
                encoding: Vec::new(),
                widths,
            });
        }
        let encoding = simple_encoding(doc, dict, budget)?;
        let widths = simple_widths(doc, dict);
        Ok(Self {
            composite,
            to_unicode,
            encoding,
            widths,
        })
    }

    /// The codes of a string operand, in order. A trailing odd byte of a composite font's
    /// string is ignored.
    pub(crate) fn codes<'f>(&'f self, bytes: &'f [u8]) -> Box<dyn Iterator<Item = Shown<'f>> + 'f> {
        if self.composite {
            Box::new(bytes.chunks_exact(2).map(|pair| {
                let code = u32::from(pair[0]) << 8 | u32::from(pair[1]);
                Shown {
                    text: self.to_unicode.get(&code).map(String::as_str),
                    width: self.width(code, None),
                    word_space: false,
                }
            }))
        } else {
            Box::new(bytes.iter().map(|byte| {
                let code = u32::from(*byte);
                let text = self.simple_text(*byte);
                Shown {
                    text,
                    width: self.width(code, text),
                    word_space: *byte == 32,
                }
            }))
        }
    }

    /// A simple font's text of a byte: the `/ToUnicode` entry, else the `/Encoding` one.
    fn simple_text(&self, byte: u8) -> Option<&str> {
        let encoded = || {
            self.encoding
                .get(usize::from(byte))
                .and_then(Option::as_ref)
        };
        self.to_unicode
            .get(&u32::from(byte))
            .or_else(encoded)
            .map(String::as_str)
    }

    fn width(&self, code: u32, text: Option<&str>) -> f64 {
        match &self.widths {
            Widths::Composite {
                default,
                codes,
                ranges,
            } => {
                let found = codes.get(&code).copied().or_else(|| {
                    ranges
                        .iter()
                        .find(|(lo, hi, _)| (*lo..=*hi).contains(&code))
                        .map(|(_, _, w)| *w)
                });
                found.unwrap_or(*default) / 1000.0
            }
            Widths::Simple {
                first,
                widths,
                missing,
                standard,
                scale,
            } => {
                let index = i64::from(code) - first;
                let listed = usize::try_from(index)
                    .ok()
                    .and_then(|i| widths.get(i))
                    .copied();
                let width = listed
                    .or_else(|| standard.and_then(|s| standard_width(s, code, text)))
                    .unwrap_or(*missing);
                width * scale
            }
        }
    }
}

fn name<'d>(dict: &'d Dictionary, key: &[u8]) -> Option<&'d [u8]> {
    dict.get(key).and_then(Object::as_name).ok()
}

fn deref<'d>(doc: &'d Document, object: &'d Object) -> &'d Object {
    match object {
        Object::Reference(id) => doc.get_object(*id).unwrap_or(object),
        other => other,
    }
}

fn get<'d>(doc: &'d Document, dict: &'d Dictionary, key: &[u8]) -> Option<&'d Object> {
    dict.get(key).ok().map(|o| deref(doc, o))
}

fn numbers(doc: &Document, object: Option<&Object>) -> Vec<f64> {
    match object {
        Some(Object::Array(items)) => items
            .iter()
            .map(|o| number(deref(doc, o)).unwrap_or(0.0))
            .collect(),
        _ => Vec::new(),
    }
}

/// The `/ToUnicode` CMap as code → text. A map that cannot be read is treated as absent.
fn to_unicode(
    doc: &Document,
    dict: &Dictionary,
    budget: &mut Budget,
) -> Result<HashMap<u32, String>, Error> {
    let Some(Object::Stream(stream)) = get(doc, dict, b"ToUnicode") else {
        return Ok(HashMap::new());
    };
    let Some(bytes) = budget.stream(stream)? else {
        return Ok(HashMap::new());
    };
    let Ok(content) = Content::decode(&bytes) else {
        return Ok(HashMap::new());
    };
    let mut map = HashMap::new();
    for op in &content.operations {
        match op.operator.as_str() {
            "endbfchar" => op
                .operands
                .chunks_exact(2)
                .for_each(|p| bf_char(&mut map, &p[0], &p[1])),
            "endbfrange" => op
                .operands
                .chunks_exact(3)
                .for_each(|t| bf_range(&mut map, &t[0], &t[1], &t[2])),
            _ => {}
        }
    }
    Ok(map)
}

/// Largest code range a `bfrange` entry may cover (two-byte codes).
const MAX_RANGE: u32 = 0xffff;

fn code(object: &Object) -> Option<u32> {
    let bytes = object.as_str().ok()?;
    (1..=4)
        .contains(&bytes.len())
        .then(|| bytes.iter().fold(0, |acc, b| acc << 8 | u32::from(*b)))
}

fn utf16(object: &Object) -> Option<Vec<u16>> {
    let bytes = object.as_str().ok()?;
    (bytes.len() % 2 == 0 && !bytes.is_empty()).then(|| {
        bytes
            .chunks_exact(2)
            .map(|p| u16::from(p[0]) << 8 | u16::from(p[1]))
            .collect()
    })
}

fn bf_char(map: &mut HashMap<u32, String>, src: &Object, dst: &Object) {
    if let (Some(code), Some(units)) = (code(src), utf16(dst))
        && let Ok(text) = String::from_utf16(&units)
    {
        map.insert(code, text);
    }
}

fn bf_range(map: &mut HashMap<u32, String>, lo: &Object, hi: &Object, dst: &Object) {
    let (Some(lo), Some(hi)) = (code(lo), code(hi)) else {
        return;
    };
    if hi < lo || hi - lo > MAX_RANGE {
        return;
    }
    if let Object::Array(targets) = dst {
        for (code, target) in (lo..=hi).zip(targets) {
            bf_char_code(map, code, target);
        }
    } else if let Some(mut units) = utf16(dst) {
        for code in lo..=hi {
            if let Ok(text) = String::from_utf16(&units) {
                map.insert(code, text);
            }
            if let Some(last) = units.last_mut() {
                *last = last.wrapping_add(1);
            }
        }
    }
}

fn bf_char_code(map: &mut HashMap<u32, String>, code: u32, dst: &Object) {
    if let Some(units) = utf16(dst)
        && let Ok(text) = String::from_utf16(&units)
    {
        map.insert(code, text);
    }
}

/// The text of every byte through the font's `/Encoding` (lopdf's tables; standard encoding
/// when absent). Without a `/ToUnicode` map, this is how a simple font's text is known.
fn simple_encoding(
    doc: &Document,
    dict: &Dictionary,
    budget: &Budget,
) -> Result<Vec<Option<String>>, Error> {
    // lopdf reads an encoding only from a dictionary typed `/Font`; some writers omit `/Type`.
    let typed;
    let dict = if dict.has_type(b"Font") {
        dict
    } else {
        typed = {
            let mut typed = dict.clone();
            typed.set("Type", Object::Name(b"Font".to_vec()));
            typed
        };
        &typed
    };
    let encoding = match dict.get_font_encoding_with_limit(doc, budget.remaining()) {
        Ok(encoding) => encoding,
        Err(lopdf::Error::Decompress(_)) => return Err(Budget::exceeded()),
        Err(_) => return Ok(vec![None; 256]),
    };
    Ok((0..=255u8)
        .map(|byte| {
            let text = encoding.bytes_to_string(&[byte]).ok()?;
            (!text.is_empty() && !text.contains('\u{fffd}')).then_some(text)
        })
        .collect())
}

fn simple_widths(doc: &Document, dict: &Dictionary) -> Widths {
    let first = get(doc, dict, b"FirstChar")
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0);
    let widths = numbers(doc, get(doc, dict, b"Widths"));
    let descriptor = get(doc, dict, b"FontDescriptor").and_then(|o| o.as_dict().ok());
    let missing = descriptor
        .and_then(|d| get(doc, d, b"MissingWidth"))
        .and_then(number)
        .unwrap_or(UNKNOWN_WIDTH);
    let standard = if widths.is_empty() {
        name(dict, b"BaseFont").and_then(standard)
    } else {
        None
    };
    // Type 3 glyph space maps to text space through /FontMatrix; others use 1/1000.
    let scale = match name(dict, b"Subtype") {
        Some(b"Type3") => numbers(doc, get(doc, dict, b"FontMatrix"))
            .first()
            .copied()
            .unwrap_or(0.001),
        _ => 0.001,
    };
    Widths::Simple {
        first,
        widths,
        missing,
        standard,
        scale,
    }
}

fn composite_widths(doc: &Document, dict: &Dictionary) -> Widths {
    let descendant = match get(doc, dict, b"DescendantFonts") {
        Some(Object::Array(items)) => items
            .first()
            .map(|o| deref(doc, o))
            .and_then(|o| o.as_dict().ok()),
        _ => None,
    };
    let Some(cid_font) = descendant else {
        return Widths::Composite {
            default: 1000.0,
            codes: HashMap::new(),
            ranges: Vec::new(),
        };
    };
    let default = get(doc, cid_font, b"DW").and_then(number).unwrap_or(1000.0);
    match get(doc, cid_font, b"W") {
        Some(Object::Array(items)) => cid_widths(doc, items, default),
        _ => Widths::Composite {
            default,
            codes: HashMap::new(),
            ranges: Vec::new(),
        },
    }
}

/// `/W`: `c [w1 w2 …]` (consecutive codes from `c`) or `c_first c_last w`. A code listed
/// twice keeps its first width.
fn cid_widths(doc: &Document, items: &[Object], default: f64) -> Widths {
    let mut codes = HashMap::new();
    let mut ranges = Vec::new();
    let mut rest = items.iter().map(|o| deref(doc, o));
    while let Some(first) = rest.next() {
        let Some(first) = first.as_i64().ok().and_then(|v| u32::try_from(v).ok()) else {
            break;
        };
        match rest.next() {
            Some(Object::Array(list)) => {
                for (offset, w) in list.iter().enumerate() {
                    let code = first.saturating_add(u32::try_from(offset).unwrap_or(u32::MAX));
                    codes
                        .entry(code)
                        .or_insert_with(|| number(deref(doc, w)).unwrap_or(0.0));
                }
            }
            Some(last) => {
                let last = last.as_i64().ok().and_then(|v| u32::try_from(v).ok());
                match (last, rest.next().and_then(number)) {
                    (Some(last), Some(w)) => ranges.push((first, last, w)),
                    _ => break,
                }
            }
            None => break,
        }
    }
    Widths::Composite {
        default,
        codes,
        ranges,
    }
}

/// The standard font a `/BaseFont` names (subset prefix removed, common aliases included).
fn standard(base_font: &[u8]) -> Option<Standard> {
    let name = match base_font.get(6) {
        Some(b'+') if base_font[..6].iter().all(u8::is_ascii_uppercase) => &base_font[7..],
        _ => base_font,
    };
    let table = |t| Some(Standard::Table(t, false));
    match name {
        b"Helvetica" | b"Arial" | b"ArialMT" => table(standard14::HELVETICA),
        b"Helvetica-Bold" | b"Arial-Bold" | b"Arial,Bold" | b"Arial-BoldMT" => {
            table(standard14::HELVETICA_BOLD)
        }
        b"Helvetica-Oblique" | b"Arial-Italic" | b"Arial,Italic" | b"Arial-ItalicMT" => {
            table(standard14::HELVETICA_OBLIQUE)
        }
        b"Helvetica-BoldOblique"
        | b"Arial-BoldItalic"
        | b"Arial,BoldItalic"
        | b"Arial-BoldItalicMT" => table(standard14::HELVETICA_BOLD_OBLIQUE),
        b"Times-Roman" | b"TimesNewRoman" | b"TimesNewRomanPSMT" => table(standard14::TIMES_ROMAN),
        b"Times-Bold" | b"TimesNewRoman,Bold" | b"TimesNewRomanPS-BoldMT" => {
            table(standard14::TIMES_BOLD)
        }
        b"Times-Italic" | b"TimesNewRoman,Italic" | b"TimesNewRomanPS-ItalicMT" => {
            table(standard14::TIMES_ITALIC)
        }
        b"Times-BoldItalic" | b"TimesNewRoman,BoldItalic" | b"TimesNewRomanPS-BoldItalicMT" => {
            table(standard14::TIMES_BOLD_ITALIC)
        }
        b"Symbol" => Some(Standard::Table(standard14::SYMBOL, true)),
        b"ZapfDingbats" => Some(Standard::Table(standard14::ZAPF_DINGBATS, true)),
        n if n.starts_with(b"Courier") => Some(Standard::Fixed(600.0)),
        _ => None,
    }
}

fn standard_width(standard: Standard, code: u32, text: Option<&str>) -> Option<f64> {
    match standard {
        Standard::Fixed(width) => Some(width),
        Standard::Table(table, by_code) => {
            let key = if by_code {
                u16::try_from(code).ok()?
            } else {
                text?.encode_utf16().next()?
            };
            let index = table.binary_search_by_key(&key, |(k, _)| *k).ok()?;
            Some(f64::from(table[index].1))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_names_and_subset_prefixes() {
        assert!(matches!(
            standard(b"ABCDEF+Arial-BoldMT"),
            Some(Standard::Table(_, false))
        ));
        assert!(matches!(
            standard(b"Courier-Bold"),
            Some(Standard::Fixed(_))
        ));
        assert!(standard(b"Abcdef+Arial").is_none());
        assert!(standard(b"Verdana").is_none());
    }

    #[test]
    fn helvetica_metrics() {
        let helvetica = standard(b"Helvetica");
        let width = |c: &str| helvetica.and_then(|s| standard_width(s, 0, Some(c)));
        assert_eq!(width(" "), Some(278.0));
        assert_eq!(width("A"), Some(667.0));
        assert_eq!(width("é"), Some(556.0));
    }

    #[test]
    fn cid_widths_by_code_and_range() {
        let doc = Document::new();
        let items = [
            Object::Integer(1),
            Object::Array(vec![Object::Integer(500), Object::Integer(600)]),
            Object::Integer(10),
            Object::Integer(20),
            Object::Integer(700),
            Object::Integer(2),
            Object::Array(vec![Object::Integer(900)]),
        ];
        let font = Font {
            composite: true,
            to_unicode: HashMap::new(),
            encoding: Vec::new(),
            widths: cid_widths(&doc, &items, 1000.0),
        };
        let width = |code| Some((font.width(code, None) * 1000.0).round());
        assert_eq!(width(1), Some(500.0));
        // A code listed twice keeps its first width.
        assert_eq!(width(2), Some(600.0));
        assert_eq!(width(15), Some(700.0));
        assert_eq!(width(30), Some(1000.0));
    }

    #[test]
    fn bf_ranges_increment_and_arrays() {
        let hex = |b: &[u8]| Object::String(b.to_vec(), lopdf::StringFormat::Hexadecimal);
        let mut map = HashMap::new();
        bf_range(&mut map, &hex(&[0, 3]), &hex(&[0, 5]), &hex(&[0, 0x41]));
        bf_range(
            &mut map,
            &hex(&[0, 9]),
            &hex(&[0, 10]),
            &Object::Array(vec![hex(&[0, 0x7a]), hex(&[0xd8, 0])]),
        );
        assert_eq!(map.get(&3).map(String::as_str), Some("A"));
        assert_eq!(map.get(&5).map(String::as_str), Some("C"));
        assert_eq!(map.get(&9).map(String::as_str), Some("z"));
        // An unpaired surrogate is not text.
        assert_eq!(map.get(&10), None);
    }
}
