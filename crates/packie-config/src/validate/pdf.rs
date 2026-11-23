//! Checks of PDF report settings (§7.7) and of the outcome directories they use (§5.4).

use std::collections::HashSet;

use rust_decimal::Decimal;

use super::{ProblemCode, Role, Validator, count};
use crate::endpoint::EndpointConfig;
use crate::protocol::{FieldPosition, OneOrMore, PdfField, PdfReportSettings, ProtocolSettings};

impl Validator {
    pub(super) fn pdf_report(&mut self, path: &str, s: &PdfReportSettings, role: Role) {
        if role == Role::Destination {
            self.err(
                ProblemCode::EndpointKindNotAllowed,
                path.trim_end_matches(".settings").to_owned() + ".protocol",
                "pdf_report is read from files only; it cannot be a destination",
            );
        }
        self.range(path, "max_file_bytes", s.max_file_bytes, 1024..=268_435_456);
        self.range(path, "max_pages", s.max_pages, 1..=500);
        self.decimal(
            path,
            "phrase_gap",
            s.phrase_gap,
            Decimal::new(5, 1),
            Decimal::from(20),
        );
        self.range(path, "label_area_end", count(s.label_area_end.len()), 0..=8);
        for (i, text) in s.label_area_end.iter().enumerate() {
            self.text(&format!("{path}.label_area_end[{i}]"), text, 100);
        }
        self.range(path, "fields", count(s.fields.len()), 1..=200);
        let mut names = HashSet::new();
        for (i, field) in s.fields.iter().enumerate() {
            let at = format!("{path}.fields[{i}]");
            if !super::protocols::column_name_ok(&field.name) {
                self.schema(format!("{at}.name"), "must match ^[a-z][a-z0-9_]{0,40}$");
            } else if !names.insert(field.name.as_str()) {
                self.duplicate(format!("{at}.name"), "field name");
            }
            self.pdf_field(&at, field);
        }
    }

    fn pdf_field(&mut self, at: &str, field: &PdfField) {
        let labels = field.label.to_vec();
        if labels.is_empty() || labels.len() > 8 {
            self.schema(format!("{at}.label"), "must be one label or 1 to 8 labels");
        }
        for (i, label) in labels.iter().enumerate() {
            let path = match field.label {
                OneOrMore::One(_) => format!("{at}.label"),
                OneOrMore::More(_) => format!("{at}.label[{i}]"),
            };
            self.text(&path, label, 100);
        }
        if !(-1..=20).contains(&field.occurrence) {
            self.schema(format!("{at}.occurrence"), "must be from -1 to 20");
        }
        let below = field.position == FieldPosition::Below;
        for (name, set) in [
            ("line", field.line.is_some()),
            ("phrase", field.phrase.is_some()),
            ("indent", field.indent.is_some()),
        ] {
            if set && !below {
                self.schema(format!("{at}.{name}"), "only used with position below");
            }
        }
        if let Some(line) = field.line {
            self.range(at, "line", line, 1..=20);
        }
        if field
            .phrase
            .is_some_and(|phrase| !(-1..=20).contains(&phrase))
        {
            self.schema(format!("{at}.phrase"), "must be from -1 to 20");
        }
        if let Some(indent) = field.indent {
            self.decimal(at, "indent", indent, Decimal::ZERO, Decimal::from(600));
        }
        let formats = field
            .format
            .as_ref()
            .map(OneOrMore::to_vec)
            .unwrap_or_default();
        if let Err(message) =
            packie_transform::ValueType::compile(field.field_type.as_str(), &formats)
        {
            self.schema(format!("{at}.format"), message);
        }
    }

    /// A decimal setting from `min` to `max` with at most two decimal places.
    fn decimal(&mut self, path: &str, field: &str, value: Decimal, min: Decimal, max: Decimal) {
        if value < min || value > max || value.normalize().scale() > 2 {
            self.schema(
                format!("{path}.{field}"),
                format!("must be from {min} to {max}, with at most two decimal places"),
            );
        }
    }

    /// Text of 1 to `max` characters after trimming.
    fn text(&mut self, path: &str, text: &str, max: u64) {
        if !(1..=max).contains(&count(text.trim().chars().count())) {
            self.schema(path, format!("must have 1 to {max} characters"));
        }
    }

    /// `filtered_directory` and `invalid_directory` belong to `pdf_report` sources.
    pub(super) fn outcome_directories(
        &mut self,
        path: &str,
        protocol: &ProtocolSettings,
        endpoint: &EndpointConfig,
    ) {
        let by_outcome = endpoint.consumption().is_some_and(|c| c.by_outcome());
        if by_outcome && !matches!(protocol, ProtocolSettings::PdfReport(_)) {
            self.schema(
                format!("{path}.endpoint"),
                "the endpoint's filtered_directory and invalid_directory are only used by \
                 pdf_report sources",
            );
        }
    }
}
