//! A stored PDF report in the message views (`docs/spec/pdf-reports.md` §6): its extracted
//! text for the body view and the export with bodies, and its template fields for the fields
//! view. The original is read with the template of the message's channel in the active
//! configuration. Text and values are message content and are never logged.

use packie_config::{ChannelConfig, ProtocolSettings, RuntimeConfig};
use packie_core::{ProtocolKind, visible_text};
use packie_ipc::messages::{
    BodyFormat, BodyRequest, BodySelector, MessageBody, MessageRecord, ParsedFields,
};
use packie_ipc::{ErrorCode, IpcError};

use crate::handler::registry_error;
use crate::pdf::{PdfReader, ReadError};
use crate::registry::ChannelRegistry;

/// A body as the body view shows it: the store's rendering, except a PDF report's original
/// as text, which is its extracted text.
///
/// # Errors
/// The store's, and `unavailable` for a report whose text cannot be extracted.
pub(crate) async fn body(
    registry: &dyn ChannelRegistry,
    config: &RuntimeConfig,
    request: BodyRequest,
) -> Result<MessageBody, IpcError> {
    if request.which == BodySelector::Original && request.format == BodyFormat::Text {
        let record = registry
            .message(request.message_id)
            .await
            .map_err(registry_error)?;
        if record.summary.protocol == ProtocolKind::PdfReport {
            return text_body(registry, config, &record, request).await;
        }
    }
    registry.message_body(request).await.map_err(registry_error)
}

/// The extracted text with control characters (the tabs between phrases) made visible and
/// the line breaks kept; size and hash are the file's.
async fn text_body(
    registry: &dyn ChannelRegistry,
    config: &RuntimeConfig,
    record: &MessageRecord,
    request: BodyRequest,
) -> Result<MessageBody, IpcError> {
    let original = crate::fields::original(registry, record)
        .await
        .map_err(|cause| {
            IpcError::new(
                ErrorCode::Unavailable,
                format!("The message's original cannot be read: {cause}."),
            )
        })?;
    let byte_length = u64::try_from(original.len()).unwrap_or(u64::MAX);
    let text = text(config, record, original).await.map_err(|error| {
        IpcError::new(
            ErrorCode::Unavailable,
            format!(
                "The text of this report cannot be extracted: {}. The hex view shows the file.",
                error.detail
            ),
        )
    })?;
    let content = text
        .split('\n')
        .map(|line| visible_text(line.as_bytes()))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(MessageBody {
        message_id: request.message_id,
        which: request.which,
        format: BodyFormat::Text,
        content,
        byte_length,
        hash: record.content_hash.clone(),
        truncated: false,
    })
}

/// The extracted text of a report (`Page n` lines, one line per text line, a tab between
/// phrases). Without the channel's template, the default phrase gap and limits apply.
///
/// # Errors
/// Why the text cannot be extracted.
pub(crate) async fn text(
    config: &RuntimeConfig,
    record: &MessageRecord,
    original: Vec<u8>,
) -> Result<String, ReadError> {
    let reader = reader(config, record).unwrap_or_default();
    blocking(move || reader.text(&original)).await
}

/// The template's fields of a report, or why they are not listed.
pub(crate) async fn fields(
    config: &RuntimeConfig,
    record: &MessageRecord,
    original: Vec<u8>,
) -> ParsedFields {
    let Some(reader) = reader(config, record) else {
        return crate::fields::unavailable(
            "The message's channel no longer reads PDF reports in the active configuration."
                .to_owned(),
        );
    };
    match blocking(move || reader.read(&original)).await {
        Ok(report) => crate::fields::of_report(report),
        Err(error) => crate::fields::unavailable(format!(
            "The report does not read with its channel's template: {}.",
            error.detail
        )),
    }
}

/// The reader of the PDF report source of `record`'s channel in `config`; `None` when the
/// channel is gone or reads something else.
fn reader(config: &RuntimeConfig, record: &MessageRecord) -> Option<PdfReader> {
    let Some(ChannelConfig::Message(channel)) = config.channel(record.summary.channel_id) else {
        return None;
    };
    match &channel.source.protocol {
        ProtocolSettings::PdfReport(settings) => Some(PdfReader::new(settings)),
        _ => None,
    }
}

/// Runs `read` off the async workers: reading a report can take seconds.
async fn blocking<T: Send + 'static>(
    read: impl FnOnce() -> Result<T, ReadError> + Send + 'static,
) -> Result<T, ReadError> {
    tokio::task::spawn_blocking(read).await.unwrap_or_else(|_| {
        Err(ReadError {
            code: "internal".to_owned(),
            detail: "the reading task stopped".to_owned(),
        })
    })
}
