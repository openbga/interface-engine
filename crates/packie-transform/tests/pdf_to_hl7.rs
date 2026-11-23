//! End to end: the fields of a synthetic newborn screening report become an HL7 ORU^R01 that
//! carries the report itself in one `ED` OBX, the way a `pdf_report` channel with an HL7
//! destination runs it. A value the report does not have leaves its HL7 field empty.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use packie_transform::{Filters, Hl7Model, Mapping, Model, Path, PdfModel, Schema, Syntax};
use serde_json::{Value, json};

const FIELDS: [(&str, &str); 14] = [
    ("patient_name", "TEST BABY DOE"),
    ("last_name", "DOE"),
    ("dob", "1/2/2026 08:05"),
    ("gender", "Female"),
    ("medical_record", ""),
    ("mother_name", "JANE DOE"),
    ("sample_id", "SYN-0001"),
    ("kit_number", "K000001"),
    ("collected", "1/3/2026 10:00"),
    ("received", "1/4/2026 09:30"),
    ("reported", "1/6/2026 14:00"),
    ("recipient", "Example Clinic"),
    ("physician", "Dr Test Clinic"),
    ("physician_facility_code", "F0001"),
];

const FILE: &[u8] = b"%PDF-1.7 synthetic report";

const EXPECTED_ORU: &str = "MSH|^~\\&|PACKIE|LAB|LIS|HOSP|20261012120500||ORU^R01^ORU_R01|CTRL-0001|P|2.5.1\r\
PID|1||||DOE^TEST BABY||202601020805|F\r\
NK1|1|JANE DOE|MTH||||N\r\
ORC|RE||SYN-0001||||||||||||||||||Dr Test Clinic^^^^^^^^^F0001\r\
OBR|1||SYN-0001|REPORT^Laboratory report^L|||202601031000|||||||202601040930||||||||202601061400|||F|||^Example Clinic\r\
OBX|1|ED|REPORT^Laboratory report^L||^AP^PDF^Base64^JVBERi0xLjcgc3ludGhldGljIHJlcG9ydA==||||||F\r\
SPM|1|K000001|||||||||||||||202601031000|202601040930\r";

fn mapping_json() -> Value {
    let datetime = |from: &str, to: &str| json!({ "op": "datetime", "from": from, "to": to, "input": "M/d/yyyy HH:mm", "output": "yyyyMMddHHmm" });
    json!({
        "tables": { "sex": { "Female": "F", "Male": "M", "F": "F", "M": "M" } },
        "rules": [
            { "op": "constant", "value": "ORU^R01^ORU_R01", "to": "MSH.9" },
            { "op": "constant", "value": "1", "to": "PID.1" },
            { "op": "when", "condition": { "path": "field.medical_record", "op": "exists" }, "then": [
                { "op": "copy", "from": "field.medical_record", "to": "PID.3.1" },
                { "op": "constant", "value": "MR", "to": "PID.3.5" }
            ] },
            { "op": "copy", "from": "field.last_name", "to": "PID.5.1" },
            { "op": "copy", "from": "field.patient_name", "to": "PID.5.2",
              "steps": [{ "step": "strip_suffix", "from": "field.last_name" }] },
            datetime("field.dob", "PID.7"),
            { "op": "lookup", "from": "field.gender", "table": "sex", "to": "PID.8", "on_missing": "skip" },
            { "op": "when", "condition": { "path": "field.mother_name", "op": "exists" }, "then": [
                { "op": "constant", "value": "1", "to": "NK1.1" },
                { "op": "copy", "from": "field.mother_name", "to": "NK1.2" },
                { "op": "constant", "value": "MTH", "to": "NK1.3" },
                { "op": "constant", "value": "N", "to": "NK1.7" }
            ] },
            { "op": "constant", "value": "RE", "to": "ORC.1" },
            { "op": "copy", "from": "field.sample_id", "to": "ORC.3" },
            { "op": "copy", "from": "field.physician", "to": "ORC.21.1" },
            { "op": "copy", "from": "field.physician_facility_code", "to": "ORC.21.10" },
            { "op": "constant", "value": "1", "to": "OBR.1" },
            { "op": "copy", "from": "field.sample_id", "to": "OBR.3" },
            { "op": "constant", "value": "REPORT^Laboratory report^L", "to": "OBR.4" },
            datetime("field.collected", "OBR.7"),
            datetime("field.received", "OBR.14"),
            datetime("field.reported", "OBR.22"),
            { "op": "constant", "value": "F", "to": "OBR.25" },
            { "op": "copy", "from": "field.recipient", "to": "OBR.28.2" },
            { "op": "constant", "value": "1", "to": "OBX.1" },
            { "op": "constant", "value": "ED", "to": "OBX.2" },
            { "op": "constant", "value": "REPORT^Laboratory report^L", "to": "OBX.3" },
            { "op": "constant", "value": "AP", "to": "OBX.5.2" },
            { "op": "constant", "value": "PDF", "to": "OBX.5.3" },
            { "op": "constant", "value": "Base64", "to": "OBX.5.4" },
            { "op": "copy", "from": "report.file_base64", "to": "OBX.5.5", "required": true },
            { "op": "constant", "value": "F", "to": "OBX.11" },
            { "op": "constant", "value": "1", "to": "SPM.1" },
            { "op": "copy", "from": "field.kit_number", "to": "SPM.2" },
            datetime("field.collected", "SPM.17"),
            datetime("field.received", "SPM.18")
        ]
    })
}

fn schema() -> Schema {
    Schema::pdf(FIELDS.map(|(name, _)| name))
}

fn report(overrides: &[(&str, &str)]) -> PdfModel {
    let fields = FIELDS.map(|(name, value)| {
        let value = overrides
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(value, |(_, v)| *v);
        (name.to_owned(), value.to_owned())
    });
    PdfModel::new(fields, 1, FILE.to_vec())
}

fn skeleton() -> Hl7Model {
    let mut message = Hl7Model::new();
    for (path, value) in [
        ("MSH.3", "PACKIE"),
        ("MSH.4", "LAB"),
        ("MSH.5", "LIS"),
        ("MSH.6", "HOSP"),
        ("MSH.7", "20261012120500"),
        ("MSH.10", "CTRL-0001"),
        ("MSH.11", "P"),
        ("MSH.12", "2.5.1"),
    ] {
        let path = Path::parse(Syntax::Hl7, path).expect("valid path");
        message.set(&path, value).expect("generated header field");
    }
    message
}

fn map(source: &PdfModel) -> String {
    let mapping = Mapping::compile(&mapping_json(), &schema(), &Schema::hl7(), "mapping")
        .unwrap_or_else(|problems| panic!("mapping should be valid: {problems:?}"));
    let mut target = skeleton();
    mapping.apply(source, &mut target).expect("mapping applies");
    String::from_utf8(target.encode()).expect("ascii")
}

#[test]
fn a_report_becomes_an_oru_with_the_pdf_embedded() {
    let oru = map(&report(&[]));
    assert_eq!(oru, EXPECTED_ORU);
    let parsed = packie_hl7::Message::parse(oru.as_bytes()).expect("the codec accepts the output");
    assert_eq!(
        parsed.get("OBX-5.5"),
        Some("JVBERi0xLjcgc3ludGhldGljIHJlcG9ydA==")
    );
}

#[test]
fn values_the_report_lacks_stay_empty() {
    let oru = map(&report(&[
        ("medical_record", "MRN-00000"),
        ("mother_name", ""),
        ("gender", ""),
    ]));
    assert!(
        oru.contains("\rPID|1||MRN-00000^^^^MR||DOE^TEST BABY||202601020805\r"),
        "{oru}"
    );
    assert!(!oru.contains("NK1"));
    assert!(!oru.contains("\"\""), "the HL7 null is never written");
}

#[test]
fn clients_are_selected_by_facility_code() {
    let filters = Filters::compile(
        &[json!({ "path": "field.physician_facility_code", "op": "in", "value": ["F0001", "F0002"] })],
        &schema(),
        "channels[0].filters",
    )
    .expect("valid filter");
    assert!(filters.passes(&report(&[])));
    assert!(!filters.passes(&report(&[("physician_facility_code", "F9999")])));
    assert!(!filters.passes(&report(&[("physician_facility_code", "")])));
}

#[test]
fn pdf_reports_cannot_be_written_or_looped_over() {
    let problems = |mapping: Value, target: &Schema| {
        Mapping::compile(&mapping, &schema(), target, "m")
            .expect_err("invalid")
            .into_iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
    };
    let into_pdf = json!({ "rules": [{ "op": "constant", "value": "x", "to": "field.dob" }] });
    assert!(problems(into_pdf, &schema())[0].contains("PDF reports are read-only"));
    let over_fields =
        json!({ "rules": [{ "op": "for_each", "over": "field", "into": "OBX", "rules": [] }] });
    assert!(
        problems(over_fields, &Schema::hl7())[0].contains("PDF reports have no repeating record")
    );
    let unknown = json!({ "rules": [{ "op": "copy", "from": "field.weight", "to": "OBX.5" }] });
    assert!(problems(unknown, &Schema::hl7())[0].contains("unknown template field `weight`"));
}
