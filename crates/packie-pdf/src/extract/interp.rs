//! The content-stream walker: text operators, the graphics state and form XObjects.
//!
//! Only what places text is interpreted (ISO 32000-1 §8.4, §9.3–9.4): `q`/`Q`, `cm`, the text
//! state (`Tc`, `Tw`, `Tz`, `TL`, `Tf`, `Ts`), text positioning (`BT`, `Td`, `TD`, `Tm`, `T*`),
//! text showing (`Tj`, `TJ`, `'`, `"`) and `Do` for form XObjects. Glyphs that do not run left
//! to right on the displayed page (rotated or vertical text) are not placed.

use std::collections::HashMap;
use std::rc::Rc;

use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use super::font::Font;
use super::matrix::{Matrix, number};
use super::{Budget, Glyph};
use crate::{Error, Reason};

/// Deepest nesting of form XObjects and of saved graphics states.
const MAX_FORM_DEPTH: usize = 16;
const MAX_SAVED_STATES: usize = 256;

/// Glyphs of one page.
pub(super) fn page(
    doc: &Document,
    id: ObjectId,
    number: u32,
    budget: &mut Budget,
) -> Result<Vec<Glyph>, Error> {
    let unreadable = || Error::unreadable(format!("the content of page {number} cannot be read"));
    let bytes = match doc.get_page_content_with_limit(id, budget.remaining()) {
        Ok(bytes) => bytes,
        Err(error) if super::is_limit(&error) => return Err(Budget::exceeded()),
        Err(_) => return Err(unreadable()),
    };
    budget.take(bytes.len())?;
    let content = Content::decode(&bytes).map_err(|_| unreadable())?;
    let resources = Resources::of_page(doc, id);
    let mut walker = Walker {
        doc,
        budget,
        page: number,
        fonts: HashMap::new(),
        forms: Vec::new(),
        glyphs: Vec::new(),
    };
    let mut state = State::new(display_matrix(doc, id));
    walker.run(&content.operations, &resources, &mut state)?;
    Ok(walker.glyphs)
}

/// User space to the displayed page: crop box (else media box) origin, rotation, y downwards.
fn display_matrix(doc: &Document, page: ObjectId) -> Matrix {
    let rect = inherited(doc, page, b"CropBox")
        .and_then(rectangle)
        .or_else(|| inherited(doc, page, b"MediaBox").and_then(rectangle))
        .unwrap_or([0.0, 0.0, 612.0, 792.0]);
    let [llx, lly, urx, ury] = rect;
    let rotate = inherited(doc, page, b"Rotate")
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0);
    match rotate.rem_euclid(360) {
        90 => Matrix::new(0.0, 1.0, 1.0, 0.0, -lly, -llx),
        180 => Matrix::new(-1.0, 0.0, 0.0, 1.0, urx, -lly),
        270 => Matrix::new(0.0, -1.0, -1.0, 0.0, ury, urx),
        _ => Matrix::new(1.0, 0.0, 0.0, -1.0, -llx, ury),
    }
}

/// A page attribute, looked up through the page's ancestors when not on the page itself.
fn inherited<'d>(doc: &'d Document, page: ObjectId, key: &[u8]) -> Option<&'d Object> {
    let mut id = page;
    for _ in 0..MAX_FORM_DEPTH * 2 {
        let dict = doc.get_dictionary(id).ok()?;
        if let Ok(value) = dict.get(key) {
            return Some(deref(doc, value));
        }
        id = dict.get(b"Parent").and_then(Object::as_reference).ok()?;
    }
    None
}

fn rectangle(object: &Object) -> Option<[f64; 4]> {
    let Object::Array(items) = object else {
        return None;
    };
    let [x0, y0, x1, y1] = items.as_slice() else {
        return None;
    };
    let [x0, y0, x1, y1] = [number(x0)?, number(y0)?, number(x1)?, number(y1)?];
    Some([x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)])
}

fn deref<'d>(doc: &'d Document, object: &'d Object) -> &'d Object {
    match object {
        Object::Reference(id) => doc.get_object(*id).unwrap_or(object),
        other => other,
    }
}

/// The resource dictionaries in force: a page's (and its ancestors'), or a form's own.
#[derive(Clone)]
struct Resources<'d> {
    dicts: Vec<&'d Dictionary>,
}

impl<'d> Resources<'d> {
    fn of_page(doc: &'d Document, page: ObjectId) -> Self {
        let (direct, ids) = doc.get_page_resources(page).unwrap_or((None, Vec::new()));
        let referenced = ids.into_iter().filter_map(|id| doc.get_dictionary(id).ok());
        Self {
            dicts: direct.into_iter().chain(referenced).collect(),
        }
    }

    fn of_form(doc: &'d Document, form: &'d Stream, parent: &Self) -> Self {
        match form.dict.get(b"Resources").map(|o| deref(doc, o)) {
            Ok(Object::Dictionary(dict)) => Self { dicts: vec![dict] },
            _ => parent.clone(),
        }
    }

    /// The named entry of a category (`Font`, `XObject`) and its object id when indirect.
    fn entry(
        &self,
        doc: &'d Document,
        category: &[u8],
        name: &[u8],
    ) -> Option<(&'d Object, Option<ObjectId>)> {
        self.dicts.iter().find_map(|dict| {
            let group = deref(doc, dict.get(category).ok()?).as_dict().ok()?;
            let object = group.get(name).ok()?;
            Some((deref(doc, object), object.as_reference().ok()))
        })
    }
}

/// The part of the graphics state that places text.
#[derive(Clone)]
struct State {
    ctm: Matrix,
    char_spacing: f64,
    word_spacing: f64,
    horizontal_scale: f64,
    leading: f64,
    font: Option<Rc<Font>>,
    size: f64,
    rise: f64,
}

impl State {
    fn new(ctm: Matrix) -> Self {
        Self {
            ctm,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scale: 1.0,
            leading: 0.0,
            font: None,
            size: 0.0,
            rise: 0.0,
        }
    }
}

/// The text matrix and text line matrix of the current text object.
struct TextPosition {
    tm: Matrix,
    tlm: Matrix,
}

impl TextPosition {
    const START: Self = Self {
        tm: Matrix::IDENTITY,
        tlm: Matrix::IDENTITY,
    };

    fn next_line(&mut self, x: f64, y: f64) {
        self.tlm = Matrix::translate(x, y).then(&self.tlm);
        self.tm = self.tlm;
    }
}

struct Walker<'d, 'b> {
    doc: &'d Document,
    budget: &'b mut Budget,
    page: u32,
    fonts: HashMap<ObjectId, Rc<Font>>,
    forms: Vec<Option<ObjectId>>,
    glyphs: Vec<Glyph>,
}

impl<'d> Walker<'d, '_> {
    fn run(
        &mut self,
        operations: &[Operation],
        resources: &Resources<'d>,
        state: &mut State,
    ) -> Result<(), Error> {
        let mut saved: Vec<State> = Vec::new();
        let mut text = TextPosition::START;
        for op in operations {
            self.budget.tick()?;
            match op.operator.as_str() {
                "BT" => text = TextPosition::START,
                "Tf" => self.set_font(&op.operands, resources, state)?,
                "Do" => self.form(&op.operands, resources, state)?,
                _ => {
                    graphics_state(op, state, &mut saved, self.page)?;
                    text_state(op, state);
                    self.text(op, state, &mut text)?;
                }
            }
        }
        Ok(())
    }

    /// Positioning and showing operators.
    fn text(
        &mut self,
        op: &Operation,
        state: &mut State,
        text: &mut TextPosition,
    ) -> Result<(), Error> {
        let operands = op.operands.as_slice();
        match (op.operator.as_str(), operands) {
            ("Td", [x, y]) => text.next_line(number(x).unwrap_or(0.0), number(y).unwrap_or(0.0)),
            ("TD", [x, y]) => {
                let y = number(y).unwrap_or(0.0);
                state.leading = -y;
                text.next_line(number(x).unwrap_or(0.0), y);
            }
            ("Tm", _) => {
                if let Some(m) = Matrix::from_objects(operands) {
                    text.tlm = m;
                    text.tm = m;
                }
            }
            ("T*", _) => text.next_line(0.0, -state.leading),
            ("Tj", [s]) => self.show_string(s, state, text)?,
            ("'", [s]) => {
                text.next_line(0.0, -state.leading);
                self.show_string(s, state, text)?;
            }
            ("\"", [aw, ac, s]) => {
                state.word_spacing = number(aw).unwrap_or(0.0);
                state.char_spacing = number(ac).unwrap_or(0.0);
                text.next_line(0.0, -state.leading);
                self.show_string(s, state, text)?;
            }
            ("TJ", [Object::Array(items)]) => self.show_array(items, state, text)?,
            _ => {}
        }
        Ok(())
    }

    /// `TJ`: strings, and numbers that move the next glyph left by thousandths of an em.
    fn show_array(
        &mut self,
        items: &[Object],
        state: &State,
        text: &mut TextPosition,
    ) -> Result<(), Error> {
        for item in items {
            let Some(adjust) = number(item) else {
                self.show_string(item, state, text)?;
                continue;
            };
            let tx = -adjust / 1000.0 * state.size * state.horizontal_scale;
            text.tm = Matrix::translate(tx, 0.0).then(&text.tm);
        }
        Ok(())
    }

    fn set_font(
        &mut self,
        operands: &[Object],
        resources: &Resources<'d>,
        state: &mut State,
    ) -> Result<(), Error> {
        let [Object::Name(name), size] = operands else {
            return Ok(());
        };
        state.size = number(size).unwrap_or(0.0);
        state.font = match resources.entry(self.doc, b"Font", name) {
            Some((Object::Dictionary(dict), id)) => Some(self.font(dict, id)?),
            _ => None,
        };
        Ok(())
    }

    fn font(&mut self, dict: &Dictionary, id: Option<ObjectId>) -> Result<Rc<Font>, Error> {
        if let Some(font) = id.and_then(|id| self.fonts.get(&id)) {
            return Ok(Rc::clone(font));
        }
        let font = Rc::new(Font::load(self.doc, dict, self.budget)?);
        if let Some(id) = id {
            self.fonts.insert(id, Rc::clone(&font));
        }
        Ok(font)
    }

    fn show_string(
        &mut self,
        string: &Object,
        state: &State,
        text: &mut TextPosition,
    ) -> Result<(), Error> {
        let (Ok(bytes), Some(font)) = (string.as_str(), state.font.as_ref()) else {
            return Ok(());
        };
        for shown in font.codes(bytes) {
            self.budget.glyph()?;
            let params = Matrix::new(
                state.size * state.horizontal_scale,
                0.0,
                0.0,
                state.size,
                0.0,
                state.rise,
            );
            let m = params.then(&text.tm).then(&state.ctm);
            // Only a glyph that would be read must have a character: rotated text (a
            // watermark) is skipped whatever its font.
            match (placement(shown.width, &m), shown.text) {
                (Some(at), Some(chars)) => self.place(chars, at),
                (Some(_), None) => return Err(self.unmapped()),
                (None, _) => {}
            }
            let spacing = state.char_spacing
                + if shown.word_space {
                    state.word_spacing
                } else {
                    0.0
                };
            let advance = (shown.width * state.size + spacing) * state.horizontal_scale;
            text.tm = Matrix::translate(advance, 0.0).then(&text.tm);
        }
        Ok(())
    }

    fn unmapped(&self) -> Error {
        Error::new(
            Reason::NoText,
            format!("a glyph on page {} has no character mapping", self.page),
        )
    }

    /// Records a glyph at its placement.
    fn place(&mut self, chars: &str, at: Placement) {
        let Placement {
            left,
            right,
            baseline,
            size,
        } = at;
        let space = !chars.is_empty() && chars.chars().all(char::is_whitespace);
        let text: String = chars.chars().filter(|c| !c.is_control()).collect();
        if space || !text.is_empty() {
            self.glyphs.push(Glyph {
                text,
                left,
                right,
                baseline,
                size,
                space,
            });
        }
    }

    fn form(
        &mut self,
        operands: &[Object],
        resources: &Resources<'d>,
        state: &State,
    ) -> Result<(), Error> {
        let [Object::Name(name)] = operands else {
            return Ok(());
        };
        let Some((Object::Stream(stream), id)) = resources.entry(self.doc, b"XObject", name) else {
            return Ok(());
        };
        if stream.dict.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Form".as_slice()) {
            return Ok(());
        }
        let unreadable =
            || Error::unreadable(format!("a form on page {} cannot be read", self.page));
        if self.forms.len() >= MAX_FORM_DEPTH || id.is_some_and(|id| self.forms.contains(&Some(id)))
        {
            return Err(unreadable());
        }
        let bytes = self.budget.stream(stream)?.ok_or_else(unreadable)?;
        let content = Content::decode(&bytes).map_err(|_| unreadable())?;
        let matrix = match stream.dict.get(b"Matrix").map(|o| deref(self.doc, o)) {
            Ok(Object::Array(items)) => Matrix::from_objects(items).unwrap_or(Matrix::IDENTITY),
            _ => Matrix::IDENTITY,
        };
        let mut inner = state.clone();
        inner.ctm = matrix.then(&state.ctm);
        let inner_resources = Resources::of_form(self.doc, stream, resources);
        self.forms.push(id);
        let result = self.run(&content.operations, &inner_resources, &mut inner);
        self.forms.pop();
        result
    }
}

/// Where a glyph drawn with the text rendering matrix `m` sits on the displayed page.
#[derive(Clone, Copy)]
struct Placement {
    left: f64,
    right: f64,
    baseline: f64,
    size: f64,
}

/// The placement of a glyph, `None` when it does not run left to right (rotated or vertical
/// text) or its matrix is degenerate.
fn placement(width: f64, m: &Matrix) -> Option<Placement> {
    let (x0, y0) = m.apply(0.0, 0.0);
    let (x1, y1) = m.apply(width, 0.0);
    let (dx, dy) = if width > 0.0 {
        (x1 - x0, y1 - y0)
    } else {
        (m.a, m.b)
    };
    let size = m.vertical_scale();
    let horizontal = dx > 0.0 && dy.abs() <= 0.1 * dx;
    (horizontal && size > 0.0 && size.is_finite() && m.is_finite()).then_some(Placement {
        left: x0,
        right: x1.max(x0),
        baseline: y0,
        size,
    })
}

/// Graphics state operators that move text: `q`, `Q` and `cm`. A `q` that would save more
/// than [`MAX_SAVED_STATES`] makes the page unreadable: ignoring it would restore the wrong
/// state at its `Q`.
fn graphics_state(
    op: &Operation,
    state: &mut State,
    saved: &mut Vec<State>,
    page: u32,
) -> Result<(), Error> {
    match op.operator.as_str() {
        "q" if saved.len() >= MAX_SAVED_STATES => {
            return Err(Error::unreadable(format!(
                "page {page} saves more than {MAX_SAVED_STATES} graphics states"
            )));
        }
        "q" => saved.push(state.clone()),
        "Q" => {
            if let Some(previous) = saved.pop() {
                *state = previous;
            }
        }
        "cm" => {
            if let Some(m) = Matrix::from_objects(&op.operands) {
                state.ctm = m.then(&state.ctm);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Text state operators (`Tc`, `Tw`, `Tz`, `TL`, `Ts`).
fn text_state(op: &Operation, state: &mut State) {
    let [value] = op.operands.as_slice() else {
        return;
    };
    let Some(value) = number(value) else {
        return;
    };
    match op.operator.as_str() {
        "Tc" => state.char_spacing = value,
        "Tw" => state.word_spacing = value,
        "Tz" => state.horizontal_scale = value / 100.0,
        "TL" => state.leading = value,
        "Ts" => state.rise = value,
        _ => {}
    }
}
