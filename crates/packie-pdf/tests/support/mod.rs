//! Synthetic PDF reports for the conformance tests. Every value is fictitious.

#![allow(
    dead_code,
    reason = "each test binary uses a different part of the builder"
)]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![allow(clippy::float_arithmetic, reason = "page positions in points")]

use lopdf::content::{Content, Operation};
use lopdf::encryption::{EncryptionState, EncryptionVersion, Permissions};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};

/// How a run of text is encoded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Font {
    /// Standard Helvetica, no widths, WinAnsi encoding.
    Helvetica,
    /// Standard Helvetica-Bold without an encoding (standard encoding).
    HelveticaBold,
    /// `Type0`, `Identity-H`, with a `/ToUnicode` map and `/W` widths.
    Composite,
    /// `Type0`, `Identity-H`, without `/ToUnicode`: its characters cannot be known.
    CompositeUnmapped,
}

/// One text run at `x` and a baseline `y` points from the top of the displayed page.
#[derive(Debug, Clone)]
pub(crate) struct Run {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) size: f64,
    pub(crate) font: Font,
    pub(crate) text: String,
}

pub(crate) fn run(x: f64, y: f64, text: &str) -> Run {
    Run {
        x,
        y,
        size: 9.0,
        font: Font::Helvetica,
        text: text.into(),
    }
}

/// A page: runs, its rotation, and runs drawn through a form XObject.
#[derive(Debug, Clone, Default)]
pub(crate) struct Page {
    pub(crate) runs: Vec<Run>,
    pub(crate) form_runs: Vec<Run>,
    pub(crate) rotate: i64,
}

impl Page {
    pub(crate) fn new(runs: Vec<Run>) -> Self {
        Self {
            runs,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Options {
    /// Compress content streams with Flate.
    pub(crate) compress: bool,
    /// Encrypt with this user password.
    pub(crate) user_password: Option<String>,
    /// Declare more pages than the page tree holds.
    pub(crate) extra_declared_pages: i64,
}

const WIDTH: f64 = 612.0;
const HEIGHT: f64 = 792.0;

/// The characters the composite font maps; a character's code is its index + 1.
const ALPHABET: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789:/-.,éÉ";

pub(crate) fn pdf(pages: &[Page]) -> Vec<u8> {
    pdf_with(pages, &Options::default())
}

pub(crate) fn pdf_with(pages: &[Page], options: &Options) -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let resources = resources(&mut doc);
    let resources_id = doc.add_object(resources.clone());
    let kids: Vec<Object> = pages
        .iter()
        .map(|page| add_page(&mut doc, page, pages_id, resources_id, &resources, options).into())
        .collect();
    let count = kids.len() as i64 + options.extra_declared_pages;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    if let Some(password) = &options.user_password {
        encrypt(&mut doc, password);
    }
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap();
    out
}

/// One page drawn by raw content `operations`, with the builder's fonts (`F1` Helvetica to
/// `F4` composite without `/ToUnicode`) and `fonts` added to them.
pub(crate) fn pdf_raw(fonts: &Dictionary, operations: Vec<Operation>) -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut resources = resources(&mut doc);
    if let Ok(Object::Dictionary(builtin)) = resources.get_mut(b"Font") {
        builtin.extend(fonts);
    }
    let content = Stream::new(dictionary! {}, Content { operations }.encode().unwrap());
    let content_id = doc.add_object(content);
    let page = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
        "MediaBox" => vec![0.into(), 0.into(), WIDTH.into(), HEIGHT.into()],
        "Resources" => resources,
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(
            dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
        ),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap();
    out
}

/// `BT /<font> <size> Tf <matrix> Tm <bytes> Tj ET`.
pub(crate) fn show(font: &str, size: f64, matrix: [f64; 6], bytes: &[u8]) -> Vec<Operation> {
    vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec![font.into(), size.into()]),
        Operation::new(
            "Tm",
            matrix.iter().map(|v| Object::Real(*v as f32)).collect(),
        ),
        Operation::new(
            "Tj",
            vec![Object::String(
                bytes.to_vec(),
                lopdf::StringFormat::Hexadecimal,
            )],
        ),
        Operation::new("ET", vec![]),
    ]
}

fn encrypt(doc: &mut Document, user_password: &str) {
    doc.trailer.set(
        "ID",
        Object::Array(vec![
            Object::string_literal(b"0123456789abcdef".to_vec()),
            Object::string_literal(b"0123456789abcdef".to_vec()),
        ]),
    );
    let version = EncryptionVersion::V2 {
        document: doc,
        owner_password: "owner",
        user_password,
        key_length: 128,
        permissions: Permissions::all(),
    };
    let state = EncryptionState::try_from(version).unwrap();
    doc.encrypt(&state).unwrap();
}

fn resources(doc: &mut Document) -> Dictionary {
    let helvetica = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let bold = doc.add_object(
        dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica-Bold" },
    );
    let composite = composite_font(doc, true);
    let unmapped = composite_font(doc, false);
    dictionary! {
        "Font" => dictionary! { "F1" => helvetica, "F2" => bold, "F3" => composite, "F4" => unmapped },
    }
}

fn composite_font(doc: &mut Document, to_unicode: bool) -> ObjectId {
    let widths: Vec<Object> = ALPHABET.chars().map(|_| Object::Integer(556)).collect();
    let cid_font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "CIDFontType2", "BaseFont" => "ABCDEF+SyntheticSans",
        "CIDSystemInfo" => dictionary! { "Registry" => Object::string_literal("Adobe"), "Ordering" => Object::string_literal("Identity"), "Supplement" => 0 },
        "DW" => 1000, "W" => vec![Object::Integer(1), Object::Array(widths)],
    });
    let mut font = dictionary! {
        "Type" => "Font", "Subtype" => "Type0", "BaseFont" => "ABCDEF+SyntheticSans",
        "Encoding" => "Identity-H", "DescendantFonts" => vec![cid_font.into()],
    };
    if to_unicode {
        let map = doc.add_object(Stream::new(dictionary! {}, cmap().into_bytes()));
        font.set("ToUnicode", map);
    }
    doc.add_object(font)
}

fn cmap() -> String {
    let mut entries = String::new();
    for (i, c) in ALPHABET.chars().enumerate() {
        let mut units = [0u16; 2];
        let unit = c.encode_utf16(&mut units)[0];
        entries.push_str(&format!("<{:04X}> <{unit:04X}>\n", i + 1));
    }
    format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
         {} beginbfchar\n{entries}endbfchar\n\
         endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n",
        ALPHABET.chars().count()
    )
}

fn add_page(
    doc: &mut Document,
    page: &Page,
    parent: ObjectId,
    resources_id: ObjectId,
    resources: &Dictionary,
    options: &Options,
) -> ObjectId {
    let mut operations = text_operations(&page.runs, page.rotate);
    let mut page_resources = resources.clone();
    if !page.form_runs.is_empty() {
        let form_ops = text_operations(&page.form_runs, page.rotate);
        // The form is drawn 5 points lower through its matrix and an outer `cm`.
        let form = Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                "Matrix" => vec![1.into(), 0.into(), 0.into(), 1.into(), 0.into(), (-3).into()],
                "Resources" => resources_id,
            },
            Content {
                operations: form_ops,
            }
            .encode()
            .unwrap(),
        );
        let form_id = doc.add_object(form);
        page_resources.set("XObject", dictionary! { "Fm1" => form_id });
        operations.extend([
            Operation::new("q", vec![]),
            Operation::new(
                "cm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    1.into(),
                    0.into(),
                    (-2).into(),
                ],
            ),
            Operation::new("Do", vec!["Fm1".into()]),
            Operation::new("Q", vec![]),
        ]);
    }
    let mut content = Stream::new(dictionary! {}, Content { operations }.encode().unwrap());
    if options.compress {
        content.compress().unwrap();
    }
    let content_id = doc.add_object(content);
    doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => parent, "Contents" => content_id, "Rotate" => page.rotate,
        "MediaBox" => vec![0.into(), 0.into(), WIDTH.into(), HEIGHT.into()],
        "Resources" => page_resources,
    })
}

/// `BT … ET` for each run, placed so that it reads left to right on the displayed page.
fn text_operations(runs: &[Run], rotate: i64) -> Vec<Operation> {
    let mut ops = Vec::new();
    for run in runs {
        let (name, bytes) = encode(run);
        // Rotate 0: user y grows upwards. Rotate 90: displayed x is user y, displayed y is user x.
        let matrix: [f64; 6] = match rotate {
            90 => [0.0, 1.0, -1.0, 0.0, run.y, run.x],
            _ => [1.0, 0.0, 0.0, 1.0, run.x, HEIGHT - run.y],
        };
        ops.extend([
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![name.into(), run.size.into()]),
            Operation::new(
                "Tm",
                matrix.iter().map(|v| Object::Real(*v as f32)).collect(),
            ),
            Operation::new(
                "Tj",
                vec![Object::String(bytes, lopdf::StringFormat::Hexadecimal)],
            ),
            Operation::new("ET", vec![]),
        ]);
    }
    ops
}

fn encode(run: &Run) -> (&'static str, Vec<u8>) {
    match run.font {
        Font::Helvetica => ("F1", latin1(&run.text)),
        Font::HelveticaBold => ("F2", latin1(&run.text)),
        Font::Composite | Font::CompositeUnmapped => {
            let name = if run.font == Font::Composite {
                "F3"
            } else {
                "F4"
            };
            let bytes = run
                .text
                .chars()
                .flat_map(|c| {
                    let code = ALPHABET.chars().position(|a| a == c).unwrap() + 1;
                    (code as u16).to_be_bytes()
                })
                .collect();
            (name, bytes)
        }
    }
}

fn latin1(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap())
        .collect()
}

/// Width of `text` in Helvetica at `size`, generously estimated (0.6 em per character).
pub(crate) fn width(text: &str, size: f64) -> f64 {
    text.chars().count() as f64 * 0.6 * size
}

/// A label and its value on one line: the value 12 points after the label.
pub(crate) fn pair(x: f64, y: f64, label: &str, value: &str) -> Vec<Run> {
    let mut runs = vec![run(x, y, label)];
    if !value.is_empty() {
        runs.push(run(x + width(label, 9.0) + 12.0, y, value));
    }
    runs
}
