//! Damaged reports never panic and never yield a partial report (PRO-06). The fuzz target
//! `pdf_read` covers arbitrary bytes; these mutate valid synthetic reports.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![allow(clippy::float_arithmetic, reason = "page positions in points")]

mod support;

use std::sync::OnceLock;

use packie_pdf::{Field, Limits, Position, Template, extract};
use proptest::prelude::*;
use proptest::sample::Index;
use support::{Options, Page, pair, pdf_with, run};

fn report(compress: bool) -> Vec<u8> {
    let mut runs = vec![run(40.0, 40.0, "Synthetic Laboratory")];
    runs.extend(pair(40.0, 100.0, "Patient Name:", "TEST BABY DOE"));
    runs.extend(pair(300.0, 100.0, "Lab Number:", "SYN-0001"));
    runs.extend(pair(40.0, 114.0, "DOB:", "1/2/2026 08:05"));
    let mut page = Page::new(runs);
    page.form_runs.push(run(40.0, 600.0, "Report sent to:"));
    page.form_runs.push(run(40.0, 614.0, "Example Clinic"));
    pdf_with(
        &[page.clone(), page],
        &Options {
            compress,
            ..Options::default()
        },
    )
}

fn reports() -> &'static [Vec<u8>; 2] {
    static REPORTS: OnceLock<[Vec<u8>; 2]> = OnceLock::new();
    REPORTS.get_or_init(|| [report(false), report(true)])
}

fn template() -> Template {
    let field = |name: &str, label: &str, position| Field {
        name: name.into(),
        labels: vec![label.into()],
        position,
        occurrence: 0,
        first_page_only: false,
        required: false,
    };
    Template {
        fields: vec![
            field("patient_name", "Patient Name:", Position::Right),
            field("sample_id", "Lab Number:", Position::Right),
            field("dob", "DOB:", Position::Right),
            field(
                "recipient",
                "Report sent to:",
                Position::Below {
                    line: 1,
                    phrase: 0,
                    indent: 4.0,
                },
            ),
        ],
        ..Template::default()
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn mutated_reports_are_read_or_refused(
        compressed in any::<bool>(),
        edits in proptest::collection::vec((any::<Index>(), any::<u8>()), 1..6),
        cut in proptest::option::of(any::<Index>()),
    ) {
        let mut bytes = reports()[usize::from(compressed)].clone();
        for (at, byte) in edits {
            let at = at.index(bytes.len());
            bytes[at] = byte;
        }
        if let Some(cut) = cut {
            bytes.truncate(cut.index(bytes.len()));
        }
        if let Ok(text) = extract(&bytes, &Limits::default()) {
            let _ = text.render(2.0);
            let _ = template().read(&text);
        }
    }
}

#[test]
fn unmutated_reports_read() {
    for bytes in reports() {
        let report = template()
            .read(&extract(bytes, &Limits::default()).unwrap())
            .unwrap();
        assert_eq!(report.field("sample_id"), Some("SYN-0001"));
        assert_eq!(report.field("recipient"), Some("Example Clinic"));
    }
}

/// Seeds for the `pdf_read` fuzz target.
#[test]
#[ignore = "writes fuzz/corpus/pdf_read"]
fn write_fuzz_seeds() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/corpus/pdf_read");
    std::fs::create_dir_all(&dir).unwrap();
    let [plain, compressed] = reports();
    std::fs::write(dir.join("report"), plain).unwrap();
    std::fs::write(dir.join("report-compressed"), compressed).unwrap();
    let encrypted = Options {
        user_password: Some(String::new()),
        ..Options::default()
    };
    std::fs::write(
        dir.join("report-encrypted"),
        pdf_with(
            &[Page::new(vec![run(40.0, 40.0, "Lab Number: SYN-0001")])],
            &encrypted,
        ),
    )
    .unwrap();
}
