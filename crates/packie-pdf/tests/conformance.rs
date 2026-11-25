//! Synthetic reports in a newborn screening layout, their variations, and inputs that must be
//! refused (`docs/spec/pdf-reports.md`, acceptance E-08 to E-11).

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![allow(clippy::float_arithmetic, reason = "page positions in points")]

mod support;

use std::time::{Duration, Instant};

use lopdf::content::Operation;
use lopdf::dictionary;
use packie_pdf::{Error, Field, Limits, Position, Reason, Report, Template, extract};
use support::{Font, Options, Page, Run, pair, pdf, pdf_raw, pdf_with, run, show};

/// Values of the synthetic report, by field name.
const VALUES: [(&str, &str); 17] = [
    ("patient_name", "TEST BABY DOE"),
    ("kit_number", "K000001"),
    ("sample_id", "SYN-0001"),
    ("dob", "1/2/2026 08:05"),
    ("medical_record", ""),
    ("collected", "1/3/2026 10:00"),
    ("gender", "F"),
    ("gestational_age", "39 wks"),
    ("received", "1/4/2026"),
    ("mother_name", "JANE DOE"),
    ("birth_weight", "3100 gm"),
    ("reported", "1/6/2026"),
    ("recipient", "Example Clinic"),
    ("recipient_street", "12 Sample St"),
    ("physician", "Dr Test Clinic"),
    ("physician_facility_code", "F0001"),
    ("last_name", "DOE"),
];

fn value(name: &str) -> &'static str {
    VALUES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
        .unwrap()
}

/// The patient grid: three columns of label and value, four rows.
fn grid(y: f64) -> Vec<Run> {
    let rows = [
        [
            ("Patient Name:", "patient_name"),
            ("Kit Number:", "kit_number"),
            ("Lab Number:", "sample_id"),
        ],
        [
            ("DOB:", "dob"),
            ("Medical Record:", "medical_record"),
            ("Date Collected:", "collected"),
        ],
        [
            ("Gender:", "gender"),
            ("Gestational Age:", "gestational_age"),
            ("Date Received:", "received"),
        ],
        [
            ("Mother Name:", "mother_name"),
            ("Birth Weight:", "birth_weight"),
            ("Date Reported:", "reported"),
        ],
    ];
    let mut runs = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        for (c, (label, name)) in row.iter().enumerate() {
            runs.extend(pair(
                40.0 + 190.0 * c as f64,
                y + 14.0 * r as f64,
                label,
                value(name),
            ));
        }
    }
    runs
}

/// A one-page report: letterhead, grid, results table, bottom block and footer.
fn report_page() -> Page {
    let mut runs = vec![
        Run {
            size: 14.0,
            font: Font::HelveticaBold,
            ..run(40.0, 40.0, "Synthetic Newborn Screening Laboratory")
        },
        run(40.0, 58.0, "100 Example Road, Testville"),
    ];
    runs.extend(grid(100.0));
    runs.extend([
        run(40.0, 200.0, "Screening Test"),
        run(220.0, 200.0, "Analyte Results"),
        run(360.0, 200.0, "Normal Range"),
        run(480.0, 200.0, "Comments"),
        run(40.0, 214.0, "Biotinidase"),
        run(220.0, 214.0, "Normal"),
        run(480.0, 214.0, "Note: synthetic"),
        run(40.0, 600.0, "Report sent to:"),
        run(230.0, 600.0, "Birth Hospital:"),
        run(420.0, 600.0, "Physician/Clinic:"),
        run(40.0, 614.0, "Example Clinic"),
        run(230.0, 614.0, "Test Hospital"),
        run(420.0, 614.0, "Dr Test Clinic"),
        run(520.0, 614.0, "F0001"),
        run(40.0, 628.0, "12 Sample St"),
        Run {
            size: 8.0,
            ..run(40.0, 760.0, "Last Name: DOE")
        },
        Run {
            size: 8.0,
            ..run(200.0, 760.0, "Accession Number: ACC-1")
        },
        Run {
            size: 8.0,
            ..run(400.0, 760.0, "Page 1 of 1")
        },
    ]);
    Page::new(runs)
}

fn right(name: &str, label: &str) -> Field {
    Field {
        name: name.into(),
        labels: vec![label.into()],
        position: Position::Right,
        occurrence: 0,
        first_page_only: false,
        required: false,
    }
}

fn below(name: &str, label: &str, line: usize, phrase: i32) -> Field {
    Field {
        position: Position::Below {
            line,
            phrase,
            indent: 4.0,
        },
        ..right(name, label)
    }
}

fn template() -> Template {
    let fields = vec![
        right("patient_name", "Patient Name:"),
        right("kit_number", "Kit Number:"),
        Field {
            required: true,
            ..right("sample_id", "Lab Number:")
        },
        right("dob", "DOB:"),
        right("medical_record", "Medical Record:"),
        right("collected", "Date Collected:"),
        right("gender", "Gender:"),
        right("gestational_age", "Gestational Age:"),
        right("received", "Date Received:"),
        right("mother_name", "Mother Name:"),
        right("birth_weight", "Birth Weight:"),
        right("reported", "Date Reported:"),
        below("recipient", "Report sent to:", 1, 0),
        below("recipient_street", "Report sent to:", 2, 0),
        below("birth_hospital", "Birth Hospital:", 1, 0),
        below("physician", "Physician/Clinic:", 1, 0),
        below("physician_facility_code", "Physician/Clinic:", 1, 1),
        Field {
            occurrence: -1,
            ..right("last_name", "Last Name:")
        },
        right("accession_number", "Accession Number:"),
    ];
    Template {
        label_area_end: vec!["Screening Test".into()],
        fields,
        ..Template::default()
    }
}

fn read(bytes: &[u8]) -> Result<Report, Error> {
    template().read(&extract(bytes, &Limits::default())?)
}

fn assert_values(report: &Report) {
    for (name, expected) in VALUES {
        assert_eq!(report.field(name), Some(expected), "field {name}");
    }
    assert_eq!(report.field("birth_hospital"), Some("Test Hospital"));
    assert_eq!(report.field("accession_number"), Some("ACC-1"));
}

fn reason(result: Result<Report, Error>) -> (Reason, String) {
    let error = result.expect_err("the report must be refused");
    (error.reason, error.detail)
}

#[test]
fn reads_every_field_of_the_layout() {
    let report = read(&pdf(&[report_page()])).unwrap();
    assert_eq!(report.pages, 1);
    assert_values(&report);
}

#[test]
fn compressed_streams_read_the_same() {
    let bytes = pdf_with(
        &[report_page()],
        &Options {
            compress: true,
            ..Options::default()
        },
    );
    assert_values(&read(&bytes).unwrap());
}

#[test]
fn rotated_pages_read_the_same() {
    let page = Page {
        rotate: 90,
        ..report_page()
    };
    assert_values(&read(&pdf(&[page])).unwrap());
}

#[test]
fn composite_fonts_and_forms_carry_text() {
    let mut page = report_page();
    // The lab number in a Type0 font, the gender drawn through a form XObject 5 points lower.
    page.runs.retain(|r| r.text != "SYN-0001" && r.text != "F");
    let lab = page
        .runs
        .iter()
        .find(|r| r.text == "Lab Number:")
        .unwrap()
        .clone();
    page.runs.push(Run {
        font: Font::Composite,
        text: "SYN-0001".into(),
        ..run(lab.x + 80.0, lab.y, "")
    });
    page.form_runs.push(run(
        40.0 + support::width("Gender:", 9.0) + 12.0,
        128.0 - 5.0,
        "F",
    ));
    assert_values(&read(&pdf(&[page])).unwrap());
}

#[test]
fn text_view_shows_lines_and_phrases() {
    let text = extract(&pdf(&[report_page()]), &Limits::default()).unwrap();
    let view = text.render(2.0);
    assert!(view.starts_with("Page 1\nSynthetic Newborn Screening Laboratory\n"));
    assert!(view.contains("Report sent to:\tBirth Hospital:\tPhysician/Clinic:\n"));
    assert!(view.contains("Example Clinic\tTest Hospital\tDr Test Clinic\tF0001\n"));
    let layout = text.layout(2.0);
    let lab = layout[0]
        .lines
        .iter()
        .flat_map(|l| &l.phrases)
        .find(|p| p.text.starts_with("Lab Number:"))
        .unwrap();
    assert!((lab.left - 420.0).abs() < 0.01, "left edge {}", lab.left);
    assert!((lab.size - 9.0).abs() < 0.01);
}

#[test]
fn absent_labels_and_values_are_empty() {
    let mut page = report_page();
    page.runs
        .retain(|r| !["Gender:", "F", "Birth Weight:", "3100 gm"].contains(&r.text.as_str()));
    let report = read(&pdf(&[page])).unwrap();
    assert_eq!(report.field("gender"), Some(""));
    assert_eq!(report.field("birth_weight"), Some(""));
    assert_eq!(report.field("medical_record"), Some(""));
    assert_eq!(report.field("sample_id"), Some("SYN-0001"));
}

#[test]
fn alternative_labels_and_moved_columns() {
    let mut page = report_page();
    for r in &mut page.runs {
        if r.text == "Kit Number:" {
            r.text = "Kit No:".into();
        }
        if r.y < 160.0 && r.x > 400.0 {
            r.x += 6.0;
        }
    }
    let mut template = template();
    template.fields[1].labels = vec!["Kit Number:".into(), "Kit No:".into()];
    let report = template
        .read(&extract(&pdf(&[page]), &Limits::default()).unwrap())
        .unwrap();
    assert_values(&report);
}

#[test]
fn a_second_page_repeating_the_grid_agrees() {
    let second = Page::new(grid(100.0));
    let report = read(&pdf(&[report_page(), second])).unwrap();
    assert_eq!(report.pages, 2);
    assert_values(&report);
}

#[test]
fn conflicting_pages_are_refused() {
    let mut second = Page::new(grid(100.0));
    for r in &mut second.runs {
        if r.text == "SYN-0001" {
            r.text = "SYN-0002".into();
        }
    }
    let (reason, detail) = reason(read(&pdf(&[report_page(), second])));
    assert_eq!(reason, Reason::TemplateMismatch);
    assert_eq!(
        detail,
        "field \"sample_id\" has different values on pages 1 and 2"
    );
}

#[test]
fn a_first_page_field_ignores_its_label_on_later_pages() {
    // Page 2 carries only the footer, which repeats the birth date without its time.
    let footer = |page: &str| Run {
        size: 8.0,
        ..run(400.0, 760.0, &format!("Page {page} of 2   DOB: 1/2/2026"))
    };
    let mut first = report_page();
    first.runs.retain(|r| !r.text.starts_with("Page "));
    first.runs.push(footer("1"));
    let bytes = pdf(&[first, Page::new(vec![footer("2")])]);
    let (reason, detail) = reason(read(&bytes));
    assert_eq!(reason, Reason::TemplateMismatch);
    assert_eq!(
        detail,
        "field \"dob\" has different values on pages 1 and 2"
    );

    let mut template = template();
    let dob = template
        .fields
        .iter_mut()
        .find(|f| f.name == "dob")
        .unwrap();
    dob.first_page_only = true;
    let report = template
        .read(&extract(&bytes, &Limits::default()).unwrap())
        .unwrap();
    assert_eq!(report.pages, 2);
    assert_values(&report);
}

#[test]
fn unknown_label_in_the_label_area_is_refused() {
    let mut page = report_page();
    page.runs
        .extend(pair(40.0, 156.0, "Mother Phone:", "555-0100"));
    let (reason, detail) = reason(read(&pdf(&[page])));
    assert_eq!(reason, Reason::TemplateMismatch);
    assert_eq!(detail, "unknown label in the label area on page 1, line 7");
}

#[test]
fn colons_outside_the_label_area_are_not_labels() {
    // "Note: synthetic" sits in the results table, after the label area ends.
    let page = report_page();
    assert!(page.runs.iter().any(|r| r.text == "Note: synthetic"));
    assert!(read(&pdf(&[page])).is_ok());
}

#[test]
fn a_missing_required_label_is_refused() {
    let mut page = report_page();
    page.runs
        .retain(|r| r.text != "Lab Number:" && r.text != "SYN-0001");
    let (reason, detail) = reason(read(&pdf(&[page])));
    assert_eq!(reason, Reason::TemplateMismatch);
    assert_eq!(detail, "label \"Lab Number:\" not found on page 1");
}

#[test]
fn an_empty_required_value_is_refused() {
    let mut page = report_page();
    page.runs.retain(|r| r.text != "SYN-0001");
    let (_, detail) = reason(read(&pdf(&[page])));
    assert_eq!(detail, "field \"sample_id\" is empty on page 1");
}

#[test]
fn a_footer_in_one_run_splits_at_labels() {
    let mut page = report_page();
    page.runs.retain(|r| r.y < 700.0);
    page.runs.push(Run {
        size: 8.0,
        ..run(
            40.0,
            760.0,
            "Last Name: DOE Accession Number: ACC-1 DOB: 1/2/2026",
        )
    });
    let report = read(&pdf(&[page])).unwrap();
    assert_eq!(report.field("last_name"), Some("DOE"));
    assert_eq!(report.field("accession_number"), Some("ACC-1"));
    // The grid's DOB comes first in reading order.
    assert_eq!(report.field("dob"), Some("1/2/2026 08:05"));
}

#[test]
fn glyphs_without_a_character_are_refused() {
    let mut page = report_page();
    page.runs.push(Run {
        font: Font::CompositeUnmapped,
        ..run(40.0, 300.0, "ABC")
    });
    let (reason, detail) = reason(read(&pdf(&[page])));
    assert_eq!(reason, Reason::NoText);
    assert_eq!(detail, "a glyph on page 1 has no character mapping");
}

/// A horizontal line of Helvetica text at `y` points above the page's bottom edge.
fn line(y: f64, text: &str) -> Vec<Operation> {
    show("F1", 9.0, [1.0, 0.0, 0.0, 1.0, 72.0, y], text.as_bytes())
}

#[test]
fn rotated_glyphs_without_a_character_are_skipped() {
    // A diagonal watermark in a font without a character map does not stop the report.
    let mut ops = show(
        "F4",
        40.0,
        [0.7, 0.7, -0.7, 0.7, 150.0, 300.0],
        &[0, 1, 0, 2],
    );
    ops.extend(line(700.0, "Lab Number: SYN-0001"));
    let text = extract(&pdf_raw(&dictionary! {}, ops), &Limits::default()).unwrap();
    assert!(text.render(2.0).contains("Lab Number: SYN-0001"));
}

#[test]
fn a_font_without_a_type_entry_reads_through_its_encoding() {
    // `/Type /Font` is required, but some writers leave it out.
    let untyped = dictionary! {
        "F9" => dictionary! { "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding" },
    };
    let ops = show(
        "F9",
        9.0,
        [1.0, 0.0, 0.0, 1.0, 72.0, 700.0],
        b"Lab Number: SYN-0001",
    );
    let text = extract(&pdf_raw(&untyped, ops), &Limits::default()).unwrap();
    assert!(text.render(2.0).contains("Lab Number: SYN-0001"));
}

#[test]
fn too_many_saved_graphics_states_are_refused() {
    let nested = |depth: usize| {
        let mut ops: Vec<Operation> = (0..depth).map(|_| Operation::new("q", vec![])).collect();
        ops.extend(line(700.0, "Lab Number: SYN-0001"));
        ops.extend((0..depth).map(|_| Operation::new("Q", vec![])));
        extract(&pdf_raw(&dictionary! {}, ops), &Limits::default())
    };
    assert!(nested(256).is_ok());
    let error = nested(257).expect_err("more than 256 saved states");
    assert_eq!(error.reason, Reason::Unreadable);
    assert_eq!(error.detail, "page 1 saves more than 256 graphics states");
}

#[test]
fn decomposed_labels_match_composed_text() {
    let page = Page::new(pair(40.0, 100.0, "N\u{e9}e:", "VALUE"));
    let template = Template {
        fields: vec![right("born", "Ne\u{301}e:")],
        ..Template::default()
    };
    let report = template
        .read(&extract(&pdf(&[page]), &Limits::default()).unwrap())
        .unwrap();
    assert_eq!(report.field("born"), Some("VALUE"));
}

#[test]
fn a_report_without_text_is_refused() {
    let (reason, _) = reason(read(&pdf(&[Page::default()])));
    assert_eq!(reason, Reason::NoText);
}

#[test]
fn unreadable_files_are_refused() {
    for bytes in [b"not a pdf at all".as_slice(), b"", b"%PDF-1.7\n%%EOF"] {
        assert_eq!(reason(read(bytes)).0, Reason::Unreadable, "{bytes:?}");
    }
}

#[test]
fn a_password_protected_file_is_refused() {
    let options = Options {
        user_password: Some("secret".into()),
        ..Options::default()
    };
    let (reason, detail) = reason(read(&pdf_with(&[report_page()], &options)));
    assert_eq!(reason, Reason::Unreadable);
    assert_eq!(detail, "the file needs a password to open");
}

#[test]
fn a_file_restricted_only_by_permissions_opens() {
    let options = Options {
        user_password: Some(String::new()),
        ..Options::default()
    };
    assert_values(&read(&pdf_with(&[report_page()], &options)).unwrap());
}

#[test]
fn missing_pages_are_never_read_as_a_shorter_report() {
    let options = Options {
        extra_declared_pages: 1,
        ..Options::default()
    };
    let (reason, detail) = reason(read(&pdf_with(&[report_page()], &options)));
    assert_eq!(reason, Reason::Unreadable);
    assert_eq!(detail, "the file declares 2 pages; 1 can be read");
}

#[test]
fn truncated_files_are_refused_or_read_whole() {
    let bytes = pdf_with(
        &[report_page(), Page::new(grid(100.0))],
        &Options {
            compress: true,
            ..Options::default()
        },
    );
    let whole = read(&bytes).unwrap();
    for cut in (0..bytes.len()).step_by(37) {
        if let Ok(report) = read(&bytes[..cut]) {
            assert_eq!(report, whole, "cut at {cut}");
        }
    }
}

#[test]
fn limits_are_enforced() {
    let bytes = pdf(&[report_page(), Page::new(grid(100.0))]);
    let limit = |limits: Limits| extract(&bytes, &limits).map(|_| ()).unwrap_err().reason;
    assert_eq!(
        limit(Limits {
            max_pages: 1,
            ..Limits::default()
        }),
        Reason::Limits
    );
    assert_eq!(
        limit(Limits {
            max_content_bytes: 500,
            ..Limits::default()
        }),
        Reason::Limits
    );
    assert_eq!(
        limit(Limits {
            max_words: 10,
            ..Limits::default()
        }),
        Reason::Limits
    );
    let past = Instant::now().checked_sub(Duration::from_secs(1));
    let many = pdf(&[Page::new(
        (0..300)
            .map(|i| run(40.0, 20.0 + f64::from(i) * 2.5, "x"))
            .collect(),
    )]);
    let error = extract(
        &many,
        &Limits {
            deadline: past,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.reason, Reason::Limits);
}

/// The code at the right of a facility row, as the last phrase: found whether or not the
/// report prints the facility's name before it.
#[test]
fn the_last_phrase_finds_a_code_after_a_blank_name() {
    let code = Field {
        position: Position::Below {
            line: 1,
            phrase: -1,
            indent: 110.0,
        },
        ..right("physician_facility_code", "Physician/Clinic:")
    };
    let read_with = |page: Page, field: &Field| {
        let mut template = template();
        for each in &mut template.fields {
            if each.name == field.name {
                each.clone_from(field);
            }
        }
        template
            .read(&extract(&pdf(&[page]), &Limits::default()).unwrap())
            .unwrap()
    };
    let full = read_with(report_page(), &code);
    assert_eq!(full.field("physician_facility_code"), Some("F0001"));

    let mut page = report_page();
    page.runs.retain(|r| r.text != "Dr Test Clinic");
    let blank = read_with(page.clone(), &code);
    assert_eq!(blank.field("physician_facility_code"), Some("F0001"));
    assert_eq!(blank.field("physician"), Some(""));
    // Counted from the name's column, the code is not found.
    let second = below("physician_facility_code", "Physician/Clinic:", 1, 1);
    assert_eq!(
        read_with(page, &second).field("physician_facility_code"),
        Some("")
    );
}
