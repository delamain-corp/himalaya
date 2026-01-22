use std::{fmt, sync::Arc};

use clap::Parser;
use color_eyre::Result;
use email::{backend::feature::BackendFeatureSource, config::Config};
use pimalaya_tui::{
    himalaya::backend::BackendBuilder,
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use serde::Serialize;
use tracing::info;

#[allow(unused)]
use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig, envelope::arg::ids::EnvelopeIdsArgs,
    folder::arg::name::FolderNameOptionalFlag,
};

/// Represents a structured message for JSON output.
#[derive(Clone, Debug, Serialize)]
pub struct StructuredMessage {
    /// The envelope ID of the message.
    pub id: String,
    /// The message headers (From, To, Subject, Date, etc.).
    pub headers: MessageHeaders,
    /// The plain text body of the message.
    pub body: String,
}

/// Represents the headers of a message.
#[derive(Clone, Debug, Default, Serialize)]
pub struct MessageHeaders {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,
}

/// A collection of structured messages.
#[derive(Clone, Debug, Serialize)]
pub struct StructuredMessages(Vec<StructuredMessage>);

impl fmt::Display for StructuredMessages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut glue = "";
        for msg in &self.0 {
            write!(f, "{glue}")?;
            write!(f, "{}", msg.body)?;
            glue = "\n\n";
        }
        Ok(())
    }
}

/// Read a human-friendly version of the message associated to the
/// given envelope id(s).
///
/// This command allows you to read a message. When reading a message,
/// the "seen" flag is automatically applied to the corresponding
/// envelope. To prevent this behaviour, use the "--preview" flag.
#[derive(Debug, Parser)]
pub struct MessageReadCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub envelopes: EnvelopeIdsArgs,

    /// Read the message without applying the "seen" flag to its
    /// corresponding envelope.
    #[arg(long, short)]
    pub preview: bool,

    /// Read only the body of the message.
    ///
    /// All headers will be removed from the message.
    #[arg(long)]
    #[arg(conflicts_with = "headers")]
    pub no_headers: bool,

    /// List of headers that should be visible at the top of the
    /// message.
    ///
    /// If a given header is not found in the message, it will not be
    /// visible. If no header is given, defaults to the one set up in
    /// your TOML configuration file.
    #[arg(long = "header", short = 'H', value_name = "NAME")]
    #[arg(conflicts_with = "no_headers")]
    pub headers: Vec<String>,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl MessageReadCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing read message(s) command");

        let folder = &self.folder.name;
        let ids = &self.envelopes.ids;

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref(), |c: &Config, name| {
                c.account(name).ok()
            })?;

        let account_config = Arc::new(account_config);

        let backend = BackendBuilder::new(
            Arc::new(toml_account_config),
            account_config.clone(),
            |builder| {
                builder
                    .without_features()
                    .with_get_messages(BackendFeatureSource::Context)
                    .with_peek_messages(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
        .await?;

        let emails = if self.preview {
            backend.peek_messages(folder, ids).await
        } else {
            backend.get_messages(folder, ids).await
        }?;

        let mut structured_messages = Vec::new();

        for (idx, email) in emails.to_vec().iter().enumerate() {
            let tpl = email
                .to_read_tpl(&account_config, |mut tpl| {
                    if self.no_headers {
                        tpl = tpl.with_hide_all_headers();
                    } else if !self.headers.is_empty() {
                        tpl = tpl.with_show_only_headers(&self.headers);
                    }

                    tpl
                })
                .await?;

            // Extract headers from the parsed email
            let parsed = email.parsed();
            let headers = if let Ok(parsed) = parsed {
                MessageHeaders {
                    from: parsed.from().map(format_address),
                    to: parsed.to().map(format_address),
                    cc: parsed.cc().map(format_address),
                    bcc: parsed.bcc().map(format_address),
                    subject: parsed.subject().map(|s| s.to_string()),
                    date: parsed.date().map(|d| d.to_rfc3339()),
                    message_id: parsed.message_id().map(|s| s.to_string()),
                    in_reply_to: parsed.in_reply_to().as_text_list()
                        .and_then(|ids| ids.first().map(|s| s.to_string())),
                }
            } else {
                MessageHeaders::default()
            };

            // Extract body from template (the body part after headers)
            let body = extract_body_from_template(&tpl);

            // Use the envelope ID if available, otherwise use index
            let id = ids.get(idx).map(|s| s.to_string()).unwrap_or_else(|| idx.to_string());

            structured_messages.push(StructuredMessage {
                id,
                headers,
                body,
            });
        }

        printer.out(StructuredMessages(structured_messages))
    }
}

/// Formats an Address to a human-readable string.
fn format_address(addr: &mail_parser::Address) -> String {
    match addr {
        mail_parser::Address::List(addrs) => {
            addrs
                .iter()
                .filter_map(|a| {
                    match (&a.name, &a.address) {
                        (Some(name), Some(email)) => Some(format!("{} <{}>", name, email)),
                        (None, Some(email)) => Some(email.to_string()),
                        (Some(name), None) => Some(name.to_string()),
                        (None, None) => None,
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
        mail_parser::Address::Group(groups) => {
            groups
                .iter()
                .map(|g| {
                    let name = g.name.as_deref().unwrap_or("");
                    let members = g
                        .addresses
                        .iter()
                        .filter_map(|a| a.address.as_ref().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}: {};", name, members)
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

/// Extracts the body from a template string (after the header section).
fn extract_body_from_template(tpl: &str) -> String {
    // The template format has headers followed by an empty line, then the body
    if let Some(pos) = tpl.find("\n\n") {
        tpl[pos + 2..].to_string()
    } else if let Some(pos) = tpl.find("\r\n\r\n") {
        tpl[pos + 4..].to_string()
    } else {
        // If no header separator found, return the whole template as body
        tpl.to_string()
    }
}
