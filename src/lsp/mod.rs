// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! LSP server entry point for `cmakefmt`.
//!
//! Start by calling [`run`], which reads JSON-RPC messages from stdin and
//! writes responses to stdout using the `lsp-server` crate.

use std::collections::HashMap;
use std::error::Error;

use lsp_server::{Connection, Message, Request, Response};
use lsp_types::notification::Notification as _;
use lsp_types::request::Request as _;
use lsp_types::{
    InitializeParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
};

use crate::Config;

/// Start the LSP server loop, reading from stdin and writing to stdout.
pub fn run() -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    // Announce capabilities during the initialize handshake.
    let caps = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
        document_range_formatting_provider: Some(lsp_types::OneOf::Left(true)),
        ..Default::default()
    };

    let server_capabilities = serde_json::to_value(caps)?;
    let _init_params: InitializeParams =
        serde_json::from_value(connection.initialize(server_capabilities)?)?;

    // Main message loop.
    // Use String keys to avoid clippy::mutable_key_type (Uri has interior mutability).
    let mut documents: HashMap<String, String> = HashMap::new();
    main_loop(&connection, &mut documents)?;

    io_threads.join()?;
    Ok(())
}

fn main_loop(
    connection: &Connection,
    documents: &mut HashMap<String, String>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                let resp = handle_request(req, documents);
                if let Some(resp) = resp {
                    connection.sender.send(Message::Response(resp))?;
                }
            }
            Message::Notification(notif) => {
                handle_notification(notif, documents);
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn handle_request(req: Request, documents: &HashMap<String, String>) -> Option<Response> {
    use lsp_types::request::{Formatting, RangeFormatting};

    if req.method == Formatting::METHOD {
        return handle_formatting(req, documents);
    }
    if req.method == RangeFormatting::METHOD {
        return handle_range_formatting(req, documents);
    }

    // Return a MethodNotFound error for unhandled requests.
    Some(Response::new_err(
        req.id,
        lsp_server::ErrorCode::MethodNotFound as i32,
        format!("method not found: {}", req.method),
    ))
}

fn handle_notification(notif: lsp_server::Notification, documents: &mut HashMap<String, String>) {
    use lsp_types::notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
    };

    match notif.method.as_str() {
        m if m == DidOpenTextDocument::METHOD => {
            if let Ok(params) =
                serde_json::from_value::<lsp_types::DidOpenTextDocumentParams>(notif.params)
            {
                documents.insert(
                    params.text_document.uri.to_string(),
                    params.text_document.text,
                );
            }
        }
        m if m == DidChangeTextDocument::METHOD => {
            if let Ok(params) =
                serde_json::from_value::<lsp_types::DidChangeTextDocumentParams>(notif.params)
            {
                // With FULL sync, there is always exactly one content change.
                if let Some(change) = params.content_changes.into_iter().last() {
                    documents.insert(params.text_document.uri.to_string(), change.text);
                }
            }
        }
        m if m == DidCloseTextDocument::METHOD => {
            if let Ok(params) =
                serde_json::from_value::<lsp_types::DidCloseTextDocumentParams>(notif.params)
            {
                documents.remove(params.text_document.uri.as_str());
            }
        }
        _ => {}
    }
}

fn handle_formatting(req: Request, documents: &HashMap<String, String>) -> Option<Response> {
    let (id, params): (_, lsp_types::DocumentFormattingParams) =
        req.extract(lsp_types::request::Formatting::METHOD).ok()?;
    let text = documents.get(params.text_document.uri.as_str())?;
    let formatted = crate::format_source(text, &Config::default()).ok()?;

    let edit = full_document_edit(text, formatted);
    let result = serde_json::to_value(vec![edit]).ok()?;
    Some(Response::new_ok(id, result))
}

fn handle_range_formatting(req: Request, documents: &HashMap<String, String>) -> Option<Response> {
    let (id, params): (_, lsp_types::DocumentRangeFormattingParams) = req
        .extract(lsp_types::request::RangeFormatting::METHOD)
        .ok()?;
    let text = documents.get(params.text_document.uri.as_str())?;

    let range = params.range;
    let start_line = range.start.line as usize;
    let end_line = range.end.line as usize;

    // Collect the lines in range (0-based, inclusive).
    let all_lines: Vec<&str> = text.lines().collect();
    let clamped_end = end_line.min(all_lines.len().saturating_sub(1));
    let slice_lines = &all_lines[start_line..=clamped_end];
    let slice_text = slice_lines.join("\n") + "\n";

    let formatted = crate::format_source(&slice_text, &Config::default()).ok()?;

    // Compute the end character position within the range.
    let last_char = slice_lines.last().map(|l: &&str| l.len()).unwrap_or(0) as u32;

    let edit = lsp_types::TextEdit {
        range: lsp_types::Range {
            start: lsp_types::Position {
                line: start_line as u32,
                character: 0,
            },
            end: lsp_types::Position {
                line: clamped_end as u32,
                character: last_char,
            },
        },
        new_text: formatted,
    };

    let result = serde_json::to_value(vec![edit]).ok()?;
    Some(Response::new_ok(id, result))
}

/// Build a [`lsp_types::TextEdit`] that replaces the entire document.
fn full_document_edit(original: &str, formatted: String) -> lsp_types::TextEdit {
    let lines: Vec<&str> = original.lines().collect();
    let last_line = lines.len().saturating_sub(1);
    let last_char = lines.last().map(|l: &&str| l.len()).unwrap_or(0) as u32;
    lsp_types::TextEdit {
        range: lsp_types::Range {
            start: lsp_types::Position {
                line: 0,
                character: 0,
            },
            end: lsp_types::Position {
                line: last_line as u32,
                character: last_char,
            },
        },
        new_text: formatted,
    }
}
