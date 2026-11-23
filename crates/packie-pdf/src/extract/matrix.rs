//! Affine transformation matrices as PDF writes them (`[a b c d e f]`, row vectors).

use lopdf::Object;

/// `[a b c d e f]`: a point `(x, y)` maps to `(a·x + c·y + e, b·x + d·y + f)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Matrix {
    pub(crate) a: f64,
    pub(crate) b: f64,
    pub(crate) c: f64,
    pub(crate) d: f64,
    pub(crate) e: f64,
    pub(crate) f: f64,
}

impl Matrix {
    pub(crate) const IDENTITY: Self = Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    pub(crate) const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    pub(crate) const fn translate(x: f64, y: f64) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, x, y)
    }

    /// This transformation followed by `next` (PDF's `self × next`).
    pub(crate) fn then(&self, next: &Self) -> Self {
        Self {
            a: self.a * next.a + self.b * next.c,
            b: self.a * next.b + self.b * next.d,
            c: self.c * next.a + self.d * next.c,
            d: self.c * next.b + self.d * next.d,
            e: self.e * next.a + self.f * next.c + next.e,
            f: self.e * next.b + self.f * next.d + next.f,
        }
    }

    pub(crate) fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    /// Length of the transformed unit vector of the y axis: the scale of a font size.
    pub(crate) fn vertical_scale(&self) -> f64 {
        self.c.hypot(self.d)
    }

    pub(crate) fn is_finite(&self) -> bool {
        [self.a, self.b, self.c, self.d, self.e, self.f]
            .iter()
            .all(|v| v.is_finite())
    }

    /// Six numbers, as the operands of `cm` and `Tm` or a `/Matrix` array; `None` otherwise.
    pub(crate) fn from_objects(objects: &[Object]) -> Option<Self> {
        let [a, b, c, d, e, f] = objects else {
            return None;
        };
        let m = Self::new(
            number(a)?,
            number(b)?,
            number(c)?,
            number(d)?,
            number(e)?,
            number(f)?,
        );
        m.is_finite().then_some(m)
    }
}

/// A PDF number (integer or real) as `f64`; `None` for anything else or a non-finite value.
pub(crate) fn number(object: &Object) -> Option<f64> {
    let value = match object {
        Object::Integer(i) => *i as f64,
        Object::Real(r) => f64::from(*r),
        _ => return None,
    };
    value.is_finite().then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn then_applies_left_first() {
        let scale = Matrix::new(2.0, 0.0, 0.0, 2.0, 0.0, 0.0);
        let shift = Matrix::translate(10.0, 5.0);
        assert_eq!(scale.then(&shift).apply(1.0, 1.0), (12.0, 7.0));
        assert_eq!(shift.then(&scale).apply(1.0, 1.0), (22.0, 12.0));
    }

    #[test]
    fn operands_must_be_six_finite_numbers() {
        let six = [1, 0, 0, 1, 5, 6].map(Object::Integer);
        assert_eq!(
            Matrix::from_objects(&six),
            Some(Matrix::translate(5.0, 6.0))
        );
        assert_eq!(Matrix::from_objects(&six[..5]), None);
        let mut bad = six;
        bad[2] = Object::Real(f32::NAN);
        assert_eq!(Matrix::from_objects(&bad), None);
    }
}
