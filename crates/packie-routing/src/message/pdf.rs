//! PDF report sources (`docs/spec/pdf-reports.md`, `docs/spec/message-channels.md` §2.2):
//! each complete file is one message, committed whole with the identifiers its template
//! reads; the pipeline parses it on a blocking thread. The file is consumed right after the
//! commit, or, with a filtered or invalid directory, once the pipeline has decided the
//! message, into the directory of its outcome.

use std::sync::Arc;

use packie_config::{Consumption, EndpointConfig, PdfReportSettings};
use packie_core::{MessageId, MessageState, ProtocolKind, Timestamp};
use packie_engine::pdf::PdfReader;
use packie_ipc::messages::SourceFile;
use packie_transport::{FileSource, FileUnit, LinkError, Operation};

use crate::message::MessageContext;
use crate::message::pipeline::Routed;
use crate::message::source::{log_committed, log_duplicate, schedule_retry};
use crate::message::store::{Identifiers, Receipt, ReceiptOutcome, StoreError};
use crate::message::transform::Invalid;
use crate::retry::Backoff;
use crate::status::Role;
use crate::stop::{StopRx, sleep_unless_stopped, stopped};

/// Reads a report with the connector's template; `Err` is the `invalid` reason and detail.
pub(crate) fn read(reader: &PdfReader, original: &[u8]) -> Result<packie_pdf::Report, Invalid> {
    reader.read(original).map_err(|error| Invalid {
        code: error.code,
        detail: Some(error.detail),
    })
}

/// The identifiers of a report: the fields named `control_id`, `sample_id` and
/// `patient_id`.
pub(crate) fn identifiers(fields: &[(String, String)]) -> Identifiers {
    let field = |name: &str| {
        fields
            .iter()
            .find(|(n, v)| n == name && !v.is_empty())
            .map(|(_, v)| v.clone())
    };
    Identifiers {
        control_id: field("control_id"),
        sample_id: field("sample_id"),
        patient_id: field("patient_id"),
        message_type: Some("pdf_report".to_owned()),
        source_identity: None,
    }
}

/// Runs a PDF report source until the channel stops.
pub(crate) async fn run_pdf_source(
    cx: Arc<MessageContext>,
    endpoint: EndpointConfig,
    settings: PdfReportSettings,
    mut stop: StopRx,
) {
    let backoff = Backoff::new(&cx.channel.retry);
    let reader = Arc::new(PdfReader::new(&settings));
    loop {
        let failure = match cx.transports.file_source(&endpoint) {
            Err(error) => error,
            Ok(mut source) => {
                // Larger files are not read into memory; they arrive `too_large`.
                source.set_max_file_bytes(settings.max_file_bytes);
                cx.shared.connected(Role::Source, None);
                match watch(&cx, &endpoint, source.as_mut(), &reader, &mut stop).await {
                    None => return,
                    Some(error) => error,
                }
            }
        };
        let delay = schedule_retry(&cx, &backoff, &failure, false);
        if !sleep_unless_stopped(&mut stop, delay).await {
            return;
        }
    }
}

/// Takes complete files until the channel stops (`None`) or the source fails.
async fn watch(
    cx: &MessageContext,
    endpoint: &EndpointConfig,
    source: &mut dyn FileSource,
    reader: &Arc<PdfReader>,
    stop: &mut StopRx,
) -> Option<LinkError> {
    loop {
        let unit = tokio::select! {
            biased;
            _ = stopped(stop) => return None,
            unit = source.next() => unit,
        };
        let unit = match unit {
            Ok(unit) => unit,
            Err(error) => return Some(error),
        };
        if let Err(error) = take_report(cx, endpoint, source, &unit, reader).await {
            // Not committed: the file stays and is found again by a new source.
            return Some(LinkError::new(
                Operation::Read,
                format!("the report could not be committed ({error})"),
            ));
        }
    }
}

/// Where a decided report goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Processed,
    Filtered,
    Invalid,
}

/// Commits the report, runs its pipeline and consumes the file.
async fn take_report(
    cx: &MessageContext,
    endpoint: &EndpointConfig,
    source: &mut dyn FileSource,
    unit: &FileUnit,
    reader: &Arc<PdfReader>,
) -> Result<(), StoreError> {
    let Some(consumption) = endpoint.consumption() else {
        return Ok(());
    };
    let committed = if unit.too_large.is_some() {
        commit_too_large(cx, unit).await
    } else {
        let identifiers = read_identifiers(reader, unit).await;
        cx.store
            .commit_received(receipt(cx, unit, identifiers), unit.bytes.clone())
            .await
            .map(Some)
    };
    let counters = cx.shared.messages.as_ref();
    if let Some(counters) = counters {
        let unavailable = matches!(committed, Err(StoreError::Unavailable(_)));
        counters
            .store_unavailable
            .store(unavailable, std::sync::atomic::Ordering::Relaxed);
    }
    let committed = committed?;
    cx.shared.session_stable();
    if !consumption.by_outcome() {
        consume(cx, source, unit, &consumption, Outcome::Processed).await;
    }
    let outcome = match committed {
        None => Outcome::Invalid,
        Some(receipt) => after_commit(cx, &receipt, &unit.bytes).await,
    };
    if consumption.by_outcome() {
        consume(cx, source, unit, &consumption, outcome).await;
    }
    Ok(())
}

/// A file over `max_file_bytes`: one `aborted` message (`file_too_large`) records it.
async fn commit_too_large(
    cx: &MessageContext,
    unit: &FileUnit,
) -> Result<Option<ReceiptOutcome>, StoreError> {
    let message = cx
        .store
        .begin_transfer(receipt(cx, unit, identifiers(&[])), Vec::new())
        .await?;
    cx.store.abort_transfer(message, "file_too_large").await?;
    Ok(None)
}

/// The identifiers the template reads, off the I/O tasks; none when the report does not
/// parse (the pipeline records why).
async fn read_identifiers(reader: &Arc<PdfReader>, unit: &FileUnit) -> Identifiers {
    let reader = Arc::clone(reader);
    let bytes = unit.bytes.clone();
    tokio::task::spawn_blocking(move || reader.read(&bytes).ok())
        .await
        .ok()
        .flatten()
        .map_or_else(|| identifiers(&[]), |report| identifiers(&report.fields))
}

/// Counts and logs the committed message, runs its pipeline, and says where the file goes.
async fn after_commit(cx: &MessageContext, outcome: &ReceiptOutcome, bytes: &[u8]) -> Outcome {
    log_committed(outcome, bytes.len(), "none");
    if let Some(counters) = &cx.shared.messages {
        cx.shared.message_received(bytes.len());
        if outcome.state == MessageState::Duplicate {
            log_duplicate(counters, outcome.message_id);
        }
    }
    if outcome.state == MessageState::Duplicate {
        return match outcome.duplicate_of {
            Some(original) => outcome_of(cx, original).await,
            None => Outcome::Processed,
        };
    }
    match cx.route(outcome.message_id, bytes, &[]).await {
        Some(Routed::Filtered(_)) => Outcome::Filtered,
        Some(Routed::Invalid(_)) => Outcome::Invalid,
        // Routed, or a store failure that leaves the message for reconciliation.
        Some(Routed::To(_)) | None => Outcome::Processed,
    }
}

/// Where the message a duplicate repeats went.
async fn outcome_of(cx: &MessageContext, original: MessageId) -> Outcome {
    match cx.store.stored_message(original).await.map(|m| m.state) {
        Ok(MessageState::Filtered) => Outcome::Filtered,
        Ok(MessageState::Invalid | MessageState::Aborted) => Outcome::Invalid,
        _ => Outcome::Processed,
    }
}

/// Moves (or deletes, or leaves) the file per `on_consumed`; a move goes to the outcome's
/// directory. A file that cannot be consumed is logged; reading it again records a duplicate.
async fn consume(
    cx: &MessageContext,
    source: &mut dyn FileSource,
    unit: &FileUnit,
    consumption: &Consumption<'_>,
    outcome: Outcome,
) {
    let directory = match outcome {
        Outcome::Processed => None,
        Outcome::Filtered => consumption.filtered,
        Outcome::Invalid => consumption.invalid,
    };
    let result = match directory {
        Some(directory) => source.consumed_into(unit, directory).await,
        None => source.consumed(unit).await,
    };
    if let Err(error) = result {
        tracing::warn!(
            event = "file.consume_failed",
            operation = %error.operation,
            "Channel {}: a report was committed but could not be moved or deleted ({}); \
             reading it again records a duplicate",
            cx.shared.id,
            super::files::without_name(&error.cause, &unit.name)
        );
    }
}

fn receipt(cx: &MessageContext, unit: &FileUnit, mut identifiers: Identifiers) -> Receipt {
    identifiers.source_identity = Some(unit.name.clone());
    Receipt {
        channel_id: cx.shared.id,
        channel_label: cx.current().label,
        source_endpoint_id: cx.channel.source.endpoint,
        source_peer: None,
        protocol: ProtocolKind::PdfReport,
        // When the engine takes the file, not the file's modified time: an old file dropped
        // today is received today (and retention counts from today).
        received_at: Timestamp::now(),
        config_version: cx.shared.config_version(),
        duplicate_detection: cx.channel.source.duplicate_detection,
        identifiers,
        source_file: Some(SourceFile {
            name: unit.name.clone(),
            // A file over the size limit was not read: its hash is unknown.
            file_hash: if unit.too_large.is_some() {
                String::new()
            } else {
                packie_store::content_hash(&unit.bytes)
            },
            first_row: 0,
            last_row: 0,
        }),
    }
}
