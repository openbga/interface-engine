//! `packie pdf test`: reads a PDF report with the active template of a channel's source and,
//! with `--destination`, maps it as that destination would (`docs/spec/pdf-reports.md` §9).
//! Nothing is stored or sent; the engine audits the test as `pdf_report_test`.

use std::path::PathBuf;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use clap::{Args, Subcommand};
use packie_config::{ChannelConfig, ProtocolSettings};
use packie_ipc::pdf::{MAX_TEST_FILE_BYTES, PdfReportTest, PdfReportTestResult, TestDestination};
use packie_ipc::{Request, Response};
use serde_json::Value;

use crate::Failure;
use crate::client::{Target, ask, unexpected};
use crate::output::{print_json, print_line, print_lines, print_table};

#[derive(Debug, Subcommand)]
pub(crate) enum PdfCommand {
    /// Read a PDF report with a channel's template and show the fields and mapped message.
    Test(TestArgs),
}

#[derive(Debug, Args)]
pub(crate) struct TestArgs {
    /// The report file (at most 8 MiB).
    file: PathBuf,
    /// The message channel whose PDF report source reads it.
    #[arg(long)]
    channel: String,
    /// One of the channel's destinations: also show the message mapped for it.
    #[arg(long)]
    destination: Option<String>,
    /// Print the whole result as JSON, with the extracted lines and their positions.
    #[arg(long)]
    json: bool,
}

pub(crate) async fn pdf(target: &Target, command: PdfCommand) -> Result<(), Failure> {
    let PdfCommand::Test(args) = command;
    let bytes = std::fs::read(&args.file)
        .map_err(|e| Failure::invalid(format!("cannot read {}: {e}", args.file.display())))?;
    if bytes.len() > MAX_TEST_FILE_BYTES {
        return Err(Failure::invalid("the file is larger than 8 MiB"));
    }
    let client = target.connect().await?;
    let Response::Config { document } = ask(&client, Request::GetConfig).await? else {
        return Err(unexpected());
    };
    let channel = document
        .channels
        .iter()
        .find_map(|c| match c {
            ChannelConfig::Message(m) if m.id.as_str() == args.channel => Some(m),
            _ => None,
        })
        .ok_or_else(|| {
            Failure::new(format!(
                "no message channel {} in the active configuration",
                args.channel
            ))
        })?;
    let ProtocolSettings::PdfReport(template) = &channel.source.protocol else {
        return Err(Failure::new(format!(
            "channel {} does not read PDF reports",
            args.channel
        )));
    };
    let destination = match &args.destination {
        None => None,
        Some(id) => {
            let destination = channel
                .destinations
                .iter()
                .find(|d| d.id.as_str() == id)
                .ok_or_else(|| {
                    Failure::new(format!("channel {} has no destination {id}", args.channel))
                })?;
            Some(TestDestination {
                protocol: destination.protocol.kind().as_str().to_owned(),
                settings: settings_of(&destination.protocol)?,
                mapping: destination.mapping.clone(),
            })
        }
    };
    let request = Request::TestPdfReport(PdfReportTest {
        settings: serde_json::to_value(template).map_err(|e| Failure::new(e.to_string()))?,
        file: Some(STANDARD.encode(&bytes)),
        message_id: None,
        destination,
    });
    let Response::PdfReportTested(result) = ask(&client, request).await? else {
        return Err(unexpected());
    };
    if args.json {
        print_json(&result)?;
    } else {
        print_result(&result);
    }
    match (result.ok, &result.output_error) {
        (false, _) => Err(Failure::new("the report does not read with the template")),
        (true, Some(_)) => Err(Failure::new("the mapping failed")),
        (true, None) => Ok(()),
    }
}

/// The `settings` of a connector, as written in the runtime document.
fn settings_of(protocol: &ProtocolSettings) -> Result<Value, Failure> {
    let value = serde_json::to_value(protocol).map_err(|e| Failure::new(e.to_string()))?;
    Ok(value.get("settings").cloned().unwrap_or(Value::Null))
}

fn print_result(result: &PdfReportTestResult) {
    if !result.ok {
        print_line(&format!(
            "The report does not read: {} ({}).",
            result.reason_code.as_deref().unwrap_or("unknown"),
            result.reason_detail.as_deref().unwrap_or("")
        ));
        return;
    }
    let rows: Vec<Vec<String>> = result
        .fields
        .iter()
        .map(|f| vec![f.path.clone(), f.value.clone()])
        .collect();
    print_table(&["PATH", "VALUE"], &rows);
    if let Some(output) = &result.output {
        print_line("");
        print_lines(
            &output
                .split(['\r', '\n'])
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        );
    }
    if let Some(error) = &result.output_error {
        print_line(&format!("The mapping failed: {error}"));
    }
}
