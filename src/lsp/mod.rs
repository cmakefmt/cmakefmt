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

use std::time::Duration;

use crate::Config;

/// Maximum time allowed for a single formatting request before it is aborted.
const FORMAT_TIMEOUT: Duration = Duration::from_secs(10);

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

/// Run `format_source` with a timeout to prevent pathological inputs from
/// freezing the editor.
fn format_with_timeout(source: &str, config: &Config) -> Option<String> {
    let source = source.to_owned();
    let config = config.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = crate::format_source(&source, &config).ok();
        let _ = tx.send(result);
    });
    rx.recv_timeout(FORMAT_TIMEOUT).ok().flatten()
}

fn handle_formatting(req: Request, documents: &HashMap<String, String>) -> Option<Response> {
    let (id, params): (_, lsp_types::DocumentFormattingParams) =
        req.extract(lsp_types::request::Formatting::METHOD).ok()?;
    let text = documents.get(params.text_document.uri.as_str())?;
    let formatted = format_with_timeout(text, &Config::default())?;

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

    let formatted = format_with_timeout(&slice_text, &Config::default())?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_server::{Notification, Request, RequestId};

    fn docs(uri: &str, text: &str) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert(uri.to_string(), text.to_string());
        m
    }

    fn formatting_request(uri: &str) -> Request {
        let params = lsp_types::DocumentFormattingParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: uri.parse().unwrap(),
            },
            options: lsp_types::FormattingOptions {
                tab_size: 2,
                insert_spaces: true,
                ..Default::default()
            },
            work_done_progress_params: Default::default(),
        };
        Request {
            id: RequestId::from(1),
            method: lsp_types::request::Formatting::METHOD.to_string(),
            params: serde_json::to_value(params).unwrap(),
        }
    }

    fn range_formatting_request(uri: &str, start_line: u32, end_line: u32) -> Request {
        let params = lsp_types::DocumentRangeFormattingParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: uri.parse().unwrap(),
            },
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: start_line,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: end_line,
                    character: 999,
                },
            },
            options: lsp_types::FormattingOptions {
                tab_size: 2,
                insert_spaces: true,
                ..Default::default()
            },
            work_done_progress_params: Default::default(),
        };
        Request {
            id: RequestId::from(2),
            method: lsp_types::request::RangeFormatting::METHOD.to_string(),
            params: serde_json::to_value(params).unwrap(),
        }
    }

    // ── full_document_edit ────────────────────────────────────────────────

    #[test]
    fn full_document_edit_covers_entire_single_line_document() {
        let original = "message(hello)\n";
        let formatted = "message(hello)\n".to_string();
        let edit = full_document_edit(original, formatted.clone());
        assert_eq!(edit.range.start.line, 0);
        assert_eq!(edit.range.start.character, 0);
        assert_eq!(edit.range.end.line, 0);
        assert_eq!(edit.range.end.character, "message(hello)".len() as u32);
        assert_eq!(edit.new_text, formatted);
    }

    #[test]
    fn full_document_edit_covers_last_line_of_multi_line_document() {
        let original = "line_one()\nline_two()\n";
        let edit = full_document_edit(original, original.to_string());
        assert_eq!(edit.range.end.line, 1);
        assert_eq!(edit.range.end.character, "line_two()".len() as u32);
    }

    #[test]
    fn full_document_edit_handles_empty_document() {
        let edit = full_document_edit("", String::new());
        assert_eq!(edit.range.start.line, 0);
        assert_eq!(edit.range.end.line, 0);
        assert_eq!(edit.range.end.character, 0);
    }

    // ── handle_formatting ─────────────────────────────────────────────────

    #[test]
    fn handle_formatting_returns_formatted_edit() {
        let uri = "file:///test.cmake";
        let text = "MESSAGE(hello)\n";
        let resp = handle_formatting(formatting_request(uri), &docs(uri, text)).unwrap();
        assert!(resp.error.is_none());
        let edits: Vec<lsp_types::TextEdit> = serde_json::from_value(resp.result.unwrap()).unwrap();
        assert_eq!(edits.len(), 1);
        // Default config lowercases commands
        assert!(edits[0].new_text.starts_with("message("));
    }

    #[test]
    fn handle_formatting_returns_none_for_unknown_uri() {
        let resp = handle_formatting(formatting_request("file:///missing.cmake"), &HashMap::new());
        assert!(resp.is_none());
    }

    // ── handle_range_formatting ───────────────────────────────────────────

    #[test]
    fn handle_range_formatting_formats_selected_lines() {
        let uri = "file:///test.cmake";
        // Three-line document; format only line 1 (0-based)
        let text = "message(a)\nMESSAGE(b)\nmessage(c)\n";
        let resp =
            handle_range_formatting(range_formatting_request(uri, 1, 1), &docs(uri, text)).unwrap();
        assert!(resp.error.is_none());
        let edits: Vec<lsp_types::TextEdit> = serde_json::from_value(resp.result.unwrap()).unwrap();
        assert_eq!(edits.len(), 1);
        assert!(edits[0].new_text.contains("message(b)"));
        // Edit covers only the requested range
        assert_eq!(edits[0].range.start.line, 1);
        assert_eq!(edits[0].range.end.line, 1);
    }

    #[test]
    fn handle_range_formatting_returns_none_for_unknown_uri() {
        let resp = handle_range_formatting(
            range_formatting_request("file:///missing.cmake", 0, 0),
            &HashMap::new(),
        );
        assert!(resp.is_none());
    }

    // ── handle_request routing ────────────────────────────────────────────

    #[test]
    fn handle_request_returns_method_not_found_for_unknown_method() {
        let req = Request {
            id: RequestId::from(99),
            method: "unknown/method".to_string(),
            params: serde_json::Value::Null,
        };
        let resp = handle_request(req, &HashMap::new()).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(
            resp.error.unwrap().code,
            lsp_server::ErrorCode::MethodNotFound as i32
        );
    }

    // ── handle_notification ───────────────────────────────────────────────

    #[test]
    fn handle_notification_did_open_inserts_document() {
        let uri = "file:///open.cmake";
        let text = "message(hello)\n";
        let params = lsp_types::DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: uri.parse().unwrap(),
                language_id: "cmake".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let notif = Notification {
            method: lsp_types::notification::DidOpenTextDocument::METHOD.to_string(),
            params: serde_json::to_value(params).unwrap(),
        };
        let mut docs = HashMap::new();
        handle_notification(notif, &mut docs);
        assert_eq!(docs.get(uri).map(String::as_str), Some(text));
    }

    #[test]
    fn handle_notification_did_change_updates_document() {
        let uri = "file:///change.cmake";
        let mut docs = HashMap::new();
        docs.insert(uri.to_string(), "old\n".to_string());

        let params = lsp_types::DidChangeTextDocumentParams {
            text_document: lsp_types::VersionedTextDocumentIdentifier {
                uri: uri.parse().unwrap(),
                version: 2,
            },
            content_changes: vec![lsp_types::TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "new\n".to_string(),
            }],
        };
        let notif = Notification {
            method: lsp_types::notification::DidChangeTextDocument::METHOD.to_string(),
            params: serde_json::to_value(params).unwrap(),
        };
        handle_notification(notif, &mut docs);
        assert_eq!(docs.get(uri).map(String::as_str), Some("new\n"));
    }

    #[test]
    fn handle_notification_did_close_removes_document() {
        let uri = "file:///close.cmake";
        let mut docs = HashMap::new();
        docs.insert(uri.to_string(), "content\n".to_string());

        let params = lsp_types::DidCloseTextDocumentParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: uri.parse().unwrap(),
            },
        };
        let notif = Notification {
            method: lsp_types::notification::DidCloseTextDocument::METHOD.to_string(),
            params: serde_json::to_value(params).unwrap(),
        };
        handle_notification(notif, &mut docs);
        assert!(!docs.contains_key(uri));
    }

    #[test]
    fn handle_notification_ignores_unknown_method() {
        let mut docs = HashMap::new();
        let notif = Notification {
            method: "unknown/notification".to_string(),
            params: serde_json::Value::Null,
        };
        handle_notification(notif, &mut docs); // should not panic
    }
}
