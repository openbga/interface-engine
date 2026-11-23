//! Regenerates `src/extract/standard14.rs`, the glyph widths of the 14 standard PDF fonts.
//!
//! The widths come from the URW base35 AFM files (AGPL-3.0 with a font exception), which are
//! metric-compatible with the standard fonts. Run with the directory that holds them, for
//! example on Debian or Ubuntu with the `fonts-urw-base35` package:
//!
//! `PACKIE_AFM_DIR=/usr/share/fonts/type1/urw-base35 cargo test -p packie-pdf --test standard14 -- --ignored`

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use lopdf::{Dictionary, Document, Object};

/// Standard font, the URW file with its metrics, and whether widths are keyed by code
/// (symbol fonts, which use their built-in encoding) instead of by character.
const FONTS: [(&str, &str, bool); 12] = [
    ("HELVETICA", "NimbusSans-Regular", false),
    ("HELVETICA_BOLD", "NimbusSans-Bold", false),
    ("HELVETICA_OBLIQUE", "NimbusSans-Italic", false),
    ("HELVETICA_BOLD_OBLIQUE", "NimbusSans-BoldItalic", false),
    ("TIMES_ROMAN", "NimbusRoman-Regular", false),
    ("TIMES_BOLD", "NimbusRoman-Bold", false),
    ("TIMES_ITALIC", "NimbusRoman-Italic", false),
    ("TIMES_BOLD_ITALIC", "NimbusRoman-BoldItalic", false),
    ("SYMBOL", "StandardSymbolsPS", true),
    ("ZAPF_DINGBATS", "D050000L", true),
    // Courier is fixed-pitch (600); these two only confirm that.
    ("COURIER", "NimbusMonoPS-Regular", false),
    ("COURIER_BOLD", "NimbusMonoPS-Bold", false),
];

#[test]
#[ignore = "regenerates src/extract/standard14.rs from PACKIE_AFM_DIR"]
fn generate() {
    let dir = std::env::var("PACKIE_AFM_DIR").expect("PACKIE_AFM_DIR names the AFM directory");
    let reachable = reachable_characters();
    let mut out = String::from(HEADER);
    for (name, file, by_code) in FONTS {
        let afm = std::fs::read_to_string(Path::new(&dir).join(format!("{file}.afm"))).unwrap();
        let widths = parse(&afm, by_code, &reachable);
        if name.starts_with("COURIER") {
            assert!(
                widths.values().all(|w| *w == 600),
                "{file} is not fixed-pitch 600"
            );
            continue;
        }
        writeln!(out, "\npub(super) static {name}: &[(u16, u16)] = &[").unwrap();
        for chunk in widths.into_iter().collect::<Vec<_>>().chunks(8) {
            let row: Vec<String> = chunk
                .iter()
                .map(|(k, w)| format!("(0x{k:04x}, {w})"))
                .collect();
            writeln!(out, "    {},", row.join(", ")).unwrap();
        }
        out.push_str("];\n");
    }
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/extract/standard14.rs");
    std::fs::write(target, out).unwrap();
}

/// Characters a simple font can address through the encodings PDF names.
fn reachable_characters() -> Vec<u16> {
    let mut all = Vec::new();
    for name in [
        "StandardEncoding",
        "WinAnsiEncoding",
        "MacRomanEncoding",
        "PDFDocEncoding",
    ] {
        let font = font_with_encoding(Object::Name(name.as_bytes().to_vec()));
        let doc = Document::new();
        let Ok(encoding) = font.get_font_encoding(&doc) else {
            continue;
        };
        for byte in 0..=255u8 {
            let text = encoding.bytes_to_string(&[byte]).unwrap_or_default();
            all.extend(text.encode_utf16());
        }
    }
    all.sort_unstable();
    all.dedup();
    all
}

/// The character lopdf maps a glyph name to, through a one-entry `/Differences` encoding.
fn character(name: &str) -> Option<u16> {
    let differences = vec![Object::Integer(65), Object::Name(name.as_bytes().to_vec())];
    let mut encoding = Dictionary::new();
    encoding.set("Differences", Object::Array(differences));
    let font = font_with_encoding(Object::Dictionary(encoding));
    let doc = Document::new();
    let text = font
        .get_font_encoding(&doc)
        .ok()?
        .bytes_to_string(b"A")
        .ok()?;
    let mut units = text.encode_utf16();
    match (units.next(), units.next()) {
        (Some(unit), None) if !(name != "A" && unit == u16::from(b'A')) => Some(unit),
        _ => None,
    }
}

fn font_with_encoding(encoding: Object) -> Dictionary {
    let mut font = Dictionary::new();
    font.set("Type", Object::Name(b"Font".to_vec()));
    font.set("Encoding", encoding);
    font
}

fn parse(afm: &str, by_code: bool, reachable: &[u16]) -> BTreeMap<u16, u16> {
    let mut widths = BTreeMap::new();
    for line in afm.lines().filter(|l| l.starts_with("C ")) {
        let mut code = None;
        let mut width = None;
        let mut name = None;
        for part in line.split(';').map(str::trim) {
            let mut words = part.split_whitespace();
            match (words.next(), words.next()) {
                (Some("C"), Some(v)) => code = v.parse::<i32>().ok(),
                (Some("WX"), Some(v)) => width = v.parse::<f64>().ok(),
                (Some("N"), Some(v)) => name = Some(v.to_owned()),
                _ => {}
            }
        }
        let Some(width) = width else {
            continue;
        };
        let width = width.round() as u16;
        let key = if by_code {
            code.filter(|c| (0..=255).contains(c)).map(|c| c as u16)
        } else {
            name.as_deref()
                .and_then(character)
                .filter(|u| reachable.binary_search(u).is_ok())
        };
        if let Some(key) = key {
            widths.entry(key).or_insert(width);
        }
    }
    widths
}

const HEADER: &str =
    "//! Glyph advance widths, in 1/1000 em, of the standard PDF fonts without width arrays.
//!
//! Generated by `tests/standard14.rs` from the URW base35 AFM files (AGPL-3.0 with a font
//! exception), which are metric-compatible with the 14 standard fonts; do not edit. Text fonts
//! are keyed by UTF-16 code unit for the characters the standard encodings reach, symbol fonts
//! by code in their built-in encoding. Courier is fixed-pitch at 600 and has no table.
";
