//! PDF report message channels over real folders and the SQLite store
//! (`docs/spec/pdf-reports.md`, `docs/spec/message-channels.md` §2.2): one message per
//! report, a source filter on the birth hospital, an HL7 file with the report embedded for
//! each listed report, and each file moved into the directory of its outcome (acceptance
//! E-14).

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use common::*;
use packie_core::{MessageState, ProtocolKind, Timestamp};
use packie_store::Store;
use serde_json::json;

/// A one-page synthetic report: a lab number and a birth hospital code in Helvetica.
fn report(lab: &str, hospital: &str) -> Vec<u8> {
    let content = format!(
        "BT /F1 9 Tf 40 700 Td (Lab Number: {lab}) Tj ET \
         BT /F1 9 Tf 40 680 Td (Hospital Code: {hospital}) Tj ET"
    );
    format!(
        "%PDF-1.4\n1 0 obj <</Type/Catalog/Pages 2 0 R>> endobj\n\
         2 0 obj <</Type/Pages/Kids[3 0 R]/Count 1>> endobj\n\
         3 0 obj <</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R\
         /Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>> endobj\n\
         4 0 obj <</Length {}>> stream\n{content}\nendstream endobj\n\
         trailer <</Root 1 0 R>>\n%%EOF\n",
        content.len()
    )
    .into_bytes()
}

struct Rig {
    _h: Harness,
    store: Store,
    inbox: std::path::PathBuf,
    outbox: std::path::PathBuf,
    _dir: tempfile::TempDir,
}

async fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let inbox = dir.path().join("in");
    let outbox = dir.path().join("out");
    std::fs::create_dir_all(&inbox).unwrap();
    std::fs::create_dir_all(&outbox).unwrap();
    let store = open_store(dir.path(), None);
    let mut h = Harness::with_options(options_with_store(system_transports(), &store));
    let mut source = folder("in-dir", &inbox);
    source["settings"]["filtered_directory"] = json!("skipped");
    source["settings"]["invalid_directory"] = json!("errors");
    let channel = json!({
        "kind": "message", "channel_id": "reports", "label": "Reports",
        "source": {"endpoint": "in-dir", "protocol": "pdf_report", "settings": {
            "fields": [
                {"name": "sample_id", "label": "Lab Number:", "required": true},
                {"name": "birth_hospital_code", "label": "Hospital Code:"},
            ],
        }},
        "filters": [{"path": "field.birth_hospital_code", "op": "in", "value": ["F0001"]}],
        "destinations": [{
            "destination_id": "out", "label": "Out", "endpoint": "out-dir",
            "protocol": "hl7_file",
            "settings": {"file_name": "ORU_{sample_id}.hl7"},
            "mapping": {"rules": [
                {"op": "constant", "value": "ORU^R01^ORU_R01", "to": "MSH.9"},
                {"op": "copy", "from": "field.sample_id", "to": "PID.3.1"},
                {"op": "constant", "value": "ED", "to": "OBX.2"},
                {"op": "constant", "value": "^AP^PDF^Base64^", "to": "OBX.5"},
                {"op": "copy", "from": "report.file_base64", "to": "OBX.5.5"},
            ]},
        }],
    });
    h.apply(document(
        vec![source, folder("out-dir", &outbox)],
        vec![channel],
    ))
    .await;
    Rig {
        _h: h,
        store,
        inbox,
        outbox,
        _dir: dir,
    }
}

fn state_of(store: &Store, sample: &str) -> Option<MessageState> {
    records(store)
        .into_iter()
        .find(|r| r.summary.sample_id.as_deref() == Some(sample))
        .map(|r| r.summary.state)
}

#[tokio::test]
async fn reports_move_into_the_directory_of_their_outcome() {
    let rig = rig().await;
    let listed = report("SYN-0001", "F0001");
    std::fs::write(rig.inbox.join("listed.pdf"), &listed).unwrap();
    std::fs::write(rig.inbox.join("other.pdf"), report("SYN-0002", "F9999")).unwrap();
    std::fs::write(rig.inbox.join("broken.pdf"), b"not a pdf").unwrap();

    let written = rig.outbox.join("ORU_SYN-0001.hl7");
    until("the listed report is delivered", || {
        written.exists().then_some(())
    })
    .await;
    let message = std::fs::read_to_string(&written).unwrap();
    let segments: Vec<&str> = message.split_terminator('\r').collect();
    assert_eq!(segments[1], "PID|||SYN-0001");
    let obx = segments[2];
    assert!(
        obx.starts_with("OBX||ED|||^AP^PDF^Base64^JVBERi0xLjQ"),
        "the report travels in Base64: {obx}"
    );

    for (name, directory) in [
        ("listed.pdf", "processed"),
        ("other.pdf", "skipped"),
        ("broken.pdf", "errors"),
    ] {
        let moved = rig.inbox.join(directory).join(name);
        until(&format!("{name} in {directory}"), || {
            moved.exists().then_some(())
        })
        .await;
        assert!(!rig.inbox.join(name).exists());
    }
    assert!(!rig.outbox.join("ORU_SYN-0002.hl7").exists());
    assert_eq!(state_of(&rig.store, "SYN-0001"), Some(MessageState::Routed));
    assert_eq!(
        state_of(&rig.store, "SYN-0002"),
        Some(MessageState::Filtered)
    );

    let records = records(&rig.store);
    let broken = records
        .iter()
        .find(|r| r.summary.state == MessageState::Invalid)
        .expect("the unreadable file is a message");
    assert_eq!(broken.reason_code.as_deref(), Some("pdf_unreadable"));
    assert_eq!(
        broken.reason_detail.as_deref(),
        Some("the file is not a readable PDF")
    );
    for record in &records {
        assert_eq!(record.summary.protocol, ProtocolKind::PdfReport);
        assert_eq!(record.summary.message_type.as_deref(), Some("pdf_report"));
    }

    // The same filtered report again: a duplicate, moved where its original went.
    std::fs::write(rig.inbox.join("other.pdf"), report("SYN-0002", "F9999")).unwrap();
    let again = rig.inbox.join("skipped").join("other-1.pdf");
    until("the repeated report is skipped too", || {
        again.exists().then_some(())
    })
    .await;
    let duplicates = records_in(&rig.store, MessageState::Duplicate);
    assert_eq!(duplicates, 1);
}

#[tokio::test]
async fn a_report_without_its_required_label_is_invalid_with_a_detail() {
    let rig = rig().await;
    let mut bytes = report("SYN-0003", "F0001");
    let at = bytes.windows(11).position(|w| w == b"Lab Number:").unwrap();
    bytes[at..at + 11].copy_from_slice(b"Lab Numbr: ");
    std::fs::write(rig.inbox.join("mismatch.pdf"), bytes).unwrap();
    let moved = rig.inbox.join("errors").join("mismatch.pdf");
    until("the report is in errors", || moved.exists().then_some(())).await;
    let record = records(&rig.store).pop().unwrap();
    assert_eq!(record.summary.state, MessageState::Invalid);
    assert_eq!(record.reason_code.as_deref(), Some("template_mismatch"));
    assert_eq!(
        record.reason_detail.as_deref(),
        Some("label \"Lab Number:\" not found on page 1")
    );
}

fn records_in(store: &Store, state: MessageState) -> usize {
    records(store)
        .iter()
        .filter(|r| r.summary.state == state)
        .count()
}

#[tokio::test]
async fn an_old_report_dropped_now_is_received_now() {
    let rig = rig().await;
    let staged = rig.inbox.join("..").join("old.pdf");
    std::fs::write(&staged, report("SYN-0004", "F0001")).unwrap();
    let year_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(365 * 86_400);
    std::fs::File::options()
        .write(true)
        .open(&staged)
        .unwrap()
        .set_modified(year_ago)
        .unwrap();
    let before = Timestamp::now();
    std::fs::rename(&staged, rig.inbox.join("old.pdf")).unwrap();
    let received = until("the old report is stored", || {
        records(&rig.store)
            .into_iter()
            .find(|r| r.summary.sample_id.as_deref() == Some("SYN-0004"))
            .map(|r| r.summary.received_at)
    })
    .await;
    assert!(
        received >= before,
        "received {received}, dropped after {before}"
    );
}
