//! HL7 file destinations over real folders and the SQLite store
//! (`docs/spec/configuration.md` §7.8, `docs/spec/message-channels.md` §8): one mapped HL7
//! message per file, without MLLP framing, named by the template; a message over
//! `max_message_bytes` is a transform error and no file; an HL7 MLLP source's messages
//! written unchanged without a mapping.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use common::*;
use packie_core::{AckLevel, DeliveryState};
use packie_hl7::mllp;
use packie_store::Store;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

struct Rig {
    _h: Harness,
    store: Store,
    inbox: std::path::PathBuf,
    outbox: std::path::PathBuf,
    _dir: tempfile::TempDir,
}

/// Rows of `sample_id,name` become ORU messages written as `ORU_{sample_id}.hl7`.
async fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let inbox = dir.path().join("in");
    let outbox = dir.path().join("out");
    std::fs::create_dir_all(&inbox).unwrap();
    std::fs::create_dir_all(&outbox).unwrap();
    let store = open_store(dir.path(), None);
    let mut h = Harness::with_options(options_with_store(system_transports(), &store));
    let channel = json!({
        "kind": "message", "channel_id": "files", "label": "Files",
        "source": {"endpoint": "in-dir", "protocol": "delimited_file", "settings": {
            "columns": [{"name": "sample_id"}, {"name": "name"}],
        }},
        "destinations": [{
            "destination_id": "hie", "label": "HIE", "endpoint": "out-dir",
            "protocol": "hl7_file",
            "settings": {
                "file_name": "ORU_{sample_id}.hl7", "sending_application": "PACKIE",
                "sending_facility": "LAB", "processing_id": "T", "max_message_bytes": 1024,
            },
            "mapping": {"rules": [
                {"op": "constant", "value": "ORU^R01^ORU_R01", "to": "MSH.9"},
                {"op": "copy", "from": "col.sample_id", "to": "PID.3.1"},
                {"op": "copy", "from": "col.name", "to": "PID.5.1"},
            ]},
        }],
    });
    h.apply(document(
        vec![folder("in-dir", &inbox), folder("out-dir", &outbox)],
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

#[tokio::test]
async fn each_delivery_writes_one_unframed_hl7_message_per_file() {
    let rig = rig().await;
    std::fs::write(
        rig.inbox.join("batch.csv"),
        "sample_id,name\r\nSID-H1,DOE\r\nSID-H1,ROE\r\n",
    )
    .unwrap();
    let first = rig.outbox.join("ORU_SID-H1.hl7");
    let second = rig.outbox.join("ORU_SID-H1-1.hl7");
    until("both files written", || {
        (first.exists() && second.exists()).then_some(())
    })
    .await;

    let text = std::fs::read_to_string(&first).unwrap();
    let segments: Vec<&str> = text.split_terminator('\r').collect();
    assert_eq!(segments.len(), 2, "{text:?}");
    assert!(text.ends_with('\r'), "every segment ends with CR");
    assert!(!text.contains(['\x0b', '\x1c', '\n']), "no MLLP framing");
    let msh: Vec<&str> = segments[0].split('|').collect();
    assert_eq!(&msh[..3], ["MSH", "^~\\&", "PACKIE"]);
    assert_eq!(msh[3], "LAB");
    assert_eq!(msh[8], "ORU^R01^ORU_R01");
    assert_eq!(msh[10], "T");
    assert_eq!(msh[11], "2.5.1");
    assert_eq!(segments[1], "PID|||SID-H1||DOE");
    assert!(
        std::fs::read_to_string(&second)
            .unwrap()
            .contains("PID|||SID-H1||ROE"),
        "the second message is suffixed, not overwritten"
    );

    let records = until("both acknowledged", || {
        let records = records(&rig.store);
        (records.len() == 2
            && records
                .iter()
                .all(|r| r.deliveries[0].state == DeliveryState::Acknowledged))
        .then_some(records)
    })
    .await;
    for record in &records {
        assert_eq!(record.deliveries[0].ack_level, AckLevel::HandedOff);
    }
}

#[tokio::test]
async fn a_message_over_max_message_bytes_is_a_transform_error_without_a_file() {
    let rig = rig().await;
    let name = "N".repeat(2000);
    std::fs::write(
        rig.inbox.join("large.csv"),
        format!("sample_id,name\r\nSID-H2,{name}\r\n"),
    )
    .unwrap();
    let record = until("the delivery fails", || {
        records(&rig.store)
            .into_iter()
            .find(|r| r.deliveries.first().map(|d| d.state) == Some(DeliveryState::Failed))
    })
    .await;
    let delivery = &record.deliveries[0];
    assert_eq!(delivery.reason_code.as_deref(), Some("transform_error"));
    let error = delivery.last_error.as_deref().unwrap_or_default();
    assert!(error.contains("max_message_bytes is 1024"), "{error}");
    assert!(!error.contains(&name), "no message content in the error");
    assert!(std::fs::read_dir(&rig.outbox).unwrap().next().is_none());
}

#[tokio::test]
async fn an_hl7_source_passes_through_to_files_each_segment_ended() {
    let dir = tempfile::tempdir().unwrap();
    let outbox = dir.path().join("out");
    std::fs::create_dir_all(&outbox).unwrap();
    let store = open_store(dir.path(), None);
    let mut h = Harness::with_options(options_with_store(system_transports(), &store));
    let port = {
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        probe.local_addr().unwrap().port()
    };
    let channel = json!({
        "kind": "message", "channel_id": "orders", "label": "Orders",
        "source": {"endpoint": "lab-in", "protocol": "hl7_mllp", "settings": {}},
        "destinations": [{
            "destination_id": "files", "label": "Files", "endpoint": "out-dir",
            "protocol": "hl7_file", "settings": {"file_name": "{control_id}.hl7"},
        }],
    });
    h.apply(document(
        vec![listener("lab-in", port), folder("out-dir", &outbox)],
        vec![channel],
    ))
    .await;
    // The sender leaves the CR off the last segment.
    let mut body = oru("PASS-1");
    body.pop();
    let frame = mllp::encode(&body, 1 << 20).unwrap();
    let mut acknowledged = false;
    for _ in 0..100 {
        if let Ok(mut sender) = TcpStream::connect(("127.0.0.1", port)).await {
            sender.write_all(&frame).await.unwrap();
            let mut reply = [0_u8; 512];
            acknowledged = sender.read(&mut reply).await.unwrap_or(0) > 0;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(acknowledged, "the source answers");
    let written = outbox.join("PASS-1.hl7");
    until("the file is written", || written.exists().then_some(())).await;
    let mut expected = body;
    expected.push(b'\r');
    assert_eq!(std::fs::read(&written).unwrap(), expected);
}
