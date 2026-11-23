//! The PDF report test tool (`docs/spec/pdf-reports.md` §9): reads a report with a template
//! and maps it for a destination, storing and sending nothing. The template and destination
//! are checked by the document's rules first; problem paths are relative to the source
//! connector (`settings.fields[2].label`) or start with `destination.`.

use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use packie_config::{ChannelConfig, MessageChannel, ProtocolSettings, RuntimeConfig};
use packie_core::{ConnectorId, ProtocolKind};
use packie_ipc::messages::FieldValue;
use packie_ipc::pdf::{
    MAX_TEST_FILE_BYTES, PdfLine, PdfPage, PdfPhrase, PdfReportTest, PdfReportTestResult,
    SHORTEN_ABOVE, SHORTENED_TO, TestDestination,
};
use packie_ipc::{ErrorCode, IpcError, Response};
use serde_json::{Value, json};

use crate::engine::Shared;
use crate::handler::registry_error;
use crate::pdf::{PdfReader, ReadError};

/// The ids the tested definition gets; it is never stored.
const CHANNEL_ID: &str = "pdf-report-test";
const DESTINATION_ID: &str = "test";

/// `TestPdfReport`.
pub(crate) async fn test_pdf_report(
    shared: &Shared,
    test: PdfReportTest,
) -> Result<Response, IpcError> {
    let channel = definition(&test)?;
    let ProtocolSettings::PdfReport(settings) = &channel.source.protocol else {
        return Err(IpcError::new(ErrorCode::Internal, "The template was lost."));
    };
    let reader = Arc::new(PdfReader::new(settings));
    let bytes = Arc::new(report_bytes(shared, &test).await?);
    let mut result = {
        let (reader, bytes) = (Arc::clone(&reader), Arc::clone(&bytes));
        tokio::task::spawn_blocking(move || examine(&reader, &bytes))
            .await
            .map_err(|_| IpcError::new(ErrorCode::Internal, "The test task stopped."))?
    };
    if result.ok && test.destination.is_some() {
        let destination = ConnectorId::new(DESTINATION_ID)
            .map_err(|_| IpcError::new(ErrorCode::Internal, "The destination id is invalid."))?;
        let preview = shared
            .registry
            .preview_output(&channel, destination, &bytes)
            .await
            .map_err(registry_error)?;
        match preview {
            Ok(output) => result.output = Some(shorten(&String::from_utf8_lossy(&output))),
            Err(error) => result.output_error = Some(error),
        }
    }
    Ok(Response::PdfReportTested(Box::new(result)))
}

/// The report's bytes: the file sent, or a stored `pdf_report` message's original.
async fn report_bytes(shared: &Shared, test: &PdfReportTest) -> Result<Vec<u8>, IpcError> {
    let bad = |message: &str| IpcError::new(ErrorCode::BadRequest, message);
    match (&test.file, test.message_id) {
        (Some(file), None) => {
            let bytes = STANDARD
                .decode(file.trim())
                .map_err(|_| bad("The file is not Base64 text."))?;
            if bytes.len() > MAX_TEST_FILE_BYTES {
                return Err(bad("The file is larger than 8 MiB."));
            }
            Ok(bytes)
        }
        (None, Some(id)) => {
            let record = shared.registry.message(id).await.map_err(registry_error)?;
            if record.summary.protocol != ProtocolKind::PdfReport {
                return Err(bad("The message is not a PDF report."));
            }
            crate::fields::original(shared.registry.as_ref(), &record)
                .await
                .map_err(|cause| {
                    IpcError::new(
                        ErrorCode::Unavailable,
                        format!("The message's original cannot be read: {cause}."),
                    )
                })
        }
        _ => Err(bad("Give either a file or a message_id.")),
    }
}

/// Extraction, layout and fields; a failure keeps the lines that were extracted.
fn examine(reader: &PdfReader, bytes: &[u8]) -> PdfReportTestResult {
    let text = match reader.extract(bytes) {
        Ok(text) => text,
        Err(error) => return failed(error, Vec::new()),
    };
    let pages = layout(&text, reader.phrase_gap());
    match reader.fields(&text) {
        Ok(report) => {
            let mut fields: Vec<FieldValue> = report
                .fields
                .into_iter()
                .map(|(name, value)| FieldValue {
                    path: format!("field.{name}"),
                    value,
                })
                .collect();
            fields.push(FieldValue {
                path: "report.pages".to_owned(),
                value: report.pages.to_string(),
            });
            PdfReportTestResult {
                ok: true,
                reason_code: None,
                reason_detail: None,
                pages,
                fields,
                output: None,
                output_error: None,
            }
        }
        Err(error) => failed(error, pages),
    }
}

fn failed(error: ReadError, pages: Vec<PdfPage>) -> PdfReportTestResult {
    PdfReportTestResult {
        ok: false,
        reason_code: Some(error.code),
        reason_detail: Some(error.detail),
        pages,
        fields: Vec::new(),
        output: None,
        output_error: None,
    }
}

fn layout(text: &packie_pdf::Text, phrase_gap: f64) -> Vec<PdfPage> {
    text.layout(phrase_gap)
        .into_iter()
        .map(|page| PdfPage {
            number: u32::try_from(page.number).unwrap_or(u32::MAX),
            lines: page
                .lines
                .into_iter()
                .map(|line| PdfLine {
                    center: line.center,
                    phrases: line
                        .phrases
                        .into_iter()
                        .map(|p| PdfPhrase {
                            text: p.text,
                            left: p.left,
                            right: p.right,
                            size: p.size,
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

/// `text` with every value between delimiters longer than [`SHORTEN_ABOVE`] characters (the
/// embedded file) cut to its first [`SHORTENED_TO`] and its length.
fn shorten(text: &str) -> String {
    let delimiter = |c: char| {
        matches!(
            c,
            '|' | '^' | '~' | '\\' | '&' | '\r' | '\n' | ',' | ';' | '\t'
        )
    };
    let mut out = String::with_capacity(text.len().min(64 * 1024));
    let mut rest = text;
    while !rest.is_empty() {
        let end = rest.find(delimiter).unwrap_or(rest.len());
        let (value, tail) = rest.split_at(end);
        let count = value.chars().count();
        if count > SHORTEN_ABOVE {
            out.extend(value.chars().take(SHORTENED_TO));
            out.push_str(&format!("…({count} characters)"));
        } else {
            out.push_str(value);
        }
        let mut chars = tail.chars();
        if let Some(c) = chars.next() {
            out.push(c);
        }
        rest = chars.as_str();
    }
    out
}

/// The tested channel: a folder source with the template and the destination (or a
/// placeholder file destination), checked by the document's rules.
fn definition(test: &PdfReportTest) -> Result<MessageChannel, IpcError> {
    let destination = test.destination.as_ref();
    let (kind, settings) = match destination.map(|d| d.protocol.as_str()) {
        Some("hl7_mllp" | "astm" | "au") => ("tcp_client", json!({"host": "localhost", "port": 1})),
        _ => ("folder", json!({"path": "."})),
    };
    let document = json!({
        "endpoints": [
            {"endpoint_id": "pdf-test-in", "label": "PDF test", "kind": "folder", "settings": {"path": "."}},
            {"endpoint_id": "pdf-test-out", "label": "PDF test output", "kind": kind, "settings": settings},
        ],
        "channels": [{
            "kind": "message", "channel_id": CHANNEL_ID, "label": "PDF test",
            "source": {"endpoint": "pdf-test-in", "protocol": "pdf_report", "settings": test.settings},
            "destinations": [destination_json(destination)],
        }],
    });
    let config = RuntimeConfig::from_value(document).map_err(|errors| {
        let mut problems = errors.into_problems();
        for problem in &mut problems {
            problem.path = relative(&problem.path);
        }
        let count = problems.iter().filter(|p| p.is_error()).count();
        IpcError {
            message: format!(
                "The template has {count} error{}; nothing was read.",
                if count == 1 { "" } else { "s" }
            ),
            ..IpcError::invalid_config(problems)
        }
    })?;
    match config.channels.into_iter().next() {
        Some(ChannelConfig::Message(channel)) => Ok(channel),
        _ => Err(IpcError::new(
            ErrorCode::Internal,
            "The tested channel was lost.",
        )),
    }
}

fn destination_json(destination: Option<&TestDestination>) -> Value {
    let mut out = match destination {
        Some(d) => json!({
            "protocol": d.protocol, "settings": d.settings,
            "mapping": d.mapping.clone().unwrap_or(Value::Null),
        }),
        // Nothing is mapped without a destination; a channel needs one.
        None => json!({
            "protocol": "delimited_file",
            "settings": {"columns": [{"name": "unused"}], "file_name": "unused.csv"},
            "mapping": {"rules": []},
        }),
    };
    out["destination_id"] = json!(DESTINATION_ID);
    out["label"] = json!("Test");
    out["endpoint"] = json!("pdf-test-out");
    out
}

/// A problem path relative to the source connector, or under `destination.`.
fn relative(path: &str) -> String {
    let rest = path.strip_prefix("channels[0].").unwrap_or(path);
    if let Some(source) = rest.strip_prefix("source.") {
        return source.to_owned();
    }
    rest.strip_prefix("destinations[0].")
        .map_or_else(|| rest.to_owned(), |d| format!("destination.{d}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_values_are_shortened_with_their_length() {
        let long = "A".repeat(SHORTEN_ABOVE + 1);
        let shortened = shorten(&format!("OBX|1|ED|^AP^PDF^Base64^{long}||F\r"));
        assert_eq!(
            shortened,
            format!(
                "OBX|1|ED|^AP^PDF^Base64^{}…(1001 characters)||F\r",
                "A".repeat(SHORTENED_TO)
            )
        );
        assert_eq!(shorten("a,b\r\n"), "a,b\r\n");
    }

    #[test]
    fn problem_paths_are_relative_to_the_connector() {
        assert_eq!(
            relative("channels[0].source.settings.fields[1].label"),
            "settings.fields[1].label"
        );
        assert_eq!(
            relative("channels[0].destinations[0].mapping.rules[0].from"),
            "destination.mapping.rules[0].from"
        );
        assert_eq!(relative("channels[0].filters"), "filters");
    }
}
