//! PDF report reading: the whole input is the file. Text that extracts is rendered and read
//! with a template of right and below fields (alternatives, occurrences, a label area end).
#![no_main]

use std::time::{Duration, Instant};

use libfuzzer_sys::fuzz_target;
use packie_pdf::{Field, Limits, Position, Template, extract};

fn field(name: &str, labels: &[&str], position: Position, occurrence: i32) -> Field {
    Field {
        name: name.into(),
        labels: labels.iter().map(|l| (*l).into()).collect(),
        position,
        occurrence,
        required: false,
    }
}

fuzz_target!(|data: &[u8]| {
    let limits = Limits {
        max_content_bytes: 8 << 20,
        deadline: Instant::now().checked_add(Duration::from_secs(5)),
        ..Limits::default()
    };
    let Ok(text) = extract(data, &limits) else {
        return;
    };
    let _ = text.render(2.0);
    let below = |line, phrase| Position::Below { line, phrase, indent: 4.0 };
    let template = Template {
        phrase_gap: 2.0,
        label_area_end: vec!["Screening Test".into()],
        fields: vec![
            field("patient_name", &["Patient Name:"], Position::Right, 0),
            field("sample_id", &["Lab Number:", "Lab No:"], Position::Right, 0),
            field("dob", &["DOB:"], Position::Right, -1),
            field("recipient", &["Report sent to:"], below(1, 0), 0),
            field("code", &["Report sent to:"], below(2, 1), 1),
        ],
    };
    let _ = template.read(&text);
});
