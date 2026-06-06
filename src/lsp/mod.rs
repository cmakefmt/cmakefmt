// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! LSP server entry point for `cmakefmt`.
//!
//! Start by calling [`run`], which reads JSON-RPC messages from stdin and
//! writes responses to stdout using the `lsp-server` crate.

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

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
        code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(true)),
        ..Default::default()
    };

    let server_capabilities = serde_json::to_value(caps)?;
    let _init_params: InitializeParams =
        serde_json::from_value(connection.initialize(server_capabilities)?)?;

    // Main message loop.
    // Use String keys to avoid clippy::mutable_key_type (Uri has interior mutability).
    let mut documents: HashMap<String, String> = HashMap::new();
    let mut config = Config::default();
    main_loop(&connection, &mut documents, &mut config)?;

    io_threads.join()?;
    Ok(())
}

fn main_loop(
    connection: &Connection,
    documents: &mut HashMap<String, String>,
    config: &mut Config,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                let resp = handle_request(req, documents, config);
                if let Some(resp) = resp {
                    connection.sender.send(Message::Response(resp))?;
                }
            }
            Message::Notification(notif) => {
                handle_notification(notif, documents, config);
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn handle_request(
    req: Request,
    documents: &HashMap<String, String>,
    config: &Config,
) -> Option<Response> {
    use lsp_types::request::{CodeActionRequest, Formatting, RangeFormatting};

    if req.method == Formatting::METHOD {
        return handle_formatting(req, documents, config);
    }
    if req.method == RangeFormatting::METHOD {
        return handle_range_formatting(req, documents, config);
    }
    if req.method == CodeActionRequest::METHOD {
        return handle_code_action(req);
    }

    // Return a MethodNotFound error for unhandled requests.
    Some(Response::new_err(
        req.id,
        lsp_server::ErrorCode::MethodNotFound as i32,
        format!("method not found: {}", req.method),
    ))
}

fn handle_notification(
    notif: lsp_server::Notification,
    documents: &mut HashMap<String, String>,
    config: &mut Config,
) {
    use lsp_types::notification::{
        DidChangeConfiguration, DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
    };

    match notif.method.as_str() {
        m if m == DidChangeConfiguration::METHOD => {
            // Reload config from the working directory. The LSP client
            // sends this notification when the user's settings change.
            *config = Config::default();
            if let Ok(loaded) =
                Config::from_files(&Config::config_sources_for(std::path::Path::new(".")))
            {
                *config = loaded;
            }
        }
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

/// Resolve the effective config for a document from its on-disk path.
///
/// Discovers `.cmakefmt.*` (and the `.editorconfig` fallback) relative to the
/// document's own location rather than the server's working directory, so a
/// multi-root workspace formats each file with its project's config. Falls
/// back to `fallback` (the workspace/default config) for unsaved or non-`file:`
/// documents and whenever discovery fails, so behaviour never regresses.
fn config_for_document(uri: &str, fallback: &Config) -> Config {
    uri_to_path(uri)
        .and_then(|path| Config::for_file(&path).ok())
        .unwrap_or_else(|| fallback.clone())
}

/// Convert a `file://` URI to a filesystem path, percent-decoding `%XX`
/// escapes. Returns `None` for non-`file:` URIs (e.g. `untitled:`) or input
/// without a path component.
fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // After `file://` comes an optional authority then the path; for local
    // files the authority is empty so `rest` already starts with `/`. Drop a
    // leading host segment defensively if one is present.
    let path_part = match rest.find('/') {
        Some(0) => rest,
        Some(idx) => &rest[idx..],
        None => return None,
    };
    Some(PathBuf::from(percent_decode(path_part)))
}

/// Minimal percent-decoder for URI path components (`%20` → space). Invalid or
/// truncated escapes are passed through verbatim.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn handle_formatting(
    req: Request,
    documents: &HashMap<String, String>,
    config: &Config,
) -> Option<Response> {
    let id = req.id.clone();
    let (id, params): (_, lsp_types::DocumentFormattingParams) =
        match req.extract(lsp_types::request::Formatting::METHOD) {
            Ok(v) => v,
            Err(err) => {
                return Some(Response::new_err(
                    id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    format!("invalid formatting params: {err}"),
                ));
            }
        };
    let text = documents.get(params.text_document.uri.as_str())?;
    let doc_config = config_for_document(params.text_document.uri.as_str(), config);
    let formatted = format_with_timeout(text, &doc_config)?;

    let edit = full_document_edit(text, formatted);
    let result = match serde_json::to_value(vec![edit]) {
        Ok(v) => v,
        Err(err) => {
            return Some(Response::new_err(
                id,
                lsp_server::ErrorCode::InternalError as i32,
                format!("failed to serialize formatting response: {err}"),
            ));
        }
    };
    Some(Response::new_ok(id, result))
}

fn handle_range_formatting(
    req: Request,
    documents: &HashMap<String, String>,
    config: &Config,
) -> Option<Response> {
    let id = req.id.clone();
    let (id, params): (_, lsp_types::DocumentRangeFormattingParams) =
        match req.extract(lsp_types::request::RangeFormatting::METHOD) {
            Ok(v) => v,
            Err(err) => {
                return Some(Response::new_err(
                    id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    format!("invalid range formatting params: {err}"),
                ));
            }
        };
    let text = documents.get(params.text_document.uri.as_str())?;

    let range = params.range;
    let start_line = range.start.line as usize;
    let end_line = range.end.line as usize;

    // Collect the lines in range (0-based, inclusive). Stale editor requests
    // can refer to lines past EOF after a document shrinks; those ranges are
    // no-ops rather than protocol errors.
    let all_lines: Vec<&str> = text.lines().collect();
    if all_lines.is_empty() || start_line >= all_lines.len() {
        return Some(Response::new_ok(id, serde_json::json!([])));
    }
    let clamped_end = end_line.min(all_lines.len().saturating_sub(1));
    if start_line > clamped_end {
        return Some(Response::new_ok(id, serde_json::json!([])));
    }
    let slice_lines = &all_lines[start_line..=clamped_end];
    let slice_text = slice_lines.join("\n") + "\n";

    let doc_config = config_for_document(params.text_document.uri.as_str(), config);
    let formatted = format_with_timeout(&slice_text, &doc_config)?;

    // Compute the end character position within the range. LSP
    // `Position.character` is in UTF-16 code units, not UTF-8 bytes.
    let last_char = slice_lines
        .last()
        .map(|l: &&str| l.encode_utf16().count())
        .unwrap_or(0) as u32;

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

    let result = match serde_json::to_value(vec![edit]) {
        Ok(v) => v,
        Err(err) => {
            return Some(Response::new_err(
                id,
                lsp_server::ErrorCode::InternalError as i32,
                format!("failed to serialize range formatting response: {err}"),
            ));
        }
    };
    Some(Response::new_ok(id, result))
}

fn handle_code_action(req: Request) -> Option<Response> {
    let id = req.id.clone();
    let (id, params): (_, lsp_types::CodeActionParams) =
        match req.extract(lsp_types::request::CodeActionRequest::METHOD) {
            Ok(v) => v,
            Err(err) => {
                return Some(Response::new_err(
                    id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    format!("invalid code action params: {err}"),
                ));
            }
        };

    let range = params.range;
    let uri = params.text_document.uri;

    // Offer a code action to wrap the selection with cmakefmt: off/on.
    let off_edit = lsp_types::TextEdit {
        range: lsp_types::Range {
            start: lsp_types::Position {
                line: range.start.line,
                character: 0,
            },
            end: lsp_types::Position {
                line: range.start.line,
                character: 0,
            },
        },
        new_text: "# cmakefmt: off\n".to_string(),
    };

    let on_edit = lsp_types::TextEdit {
        range: lsp_types::Range {
            start: lsp_types::Position {
                line: range.end.line + 1,
                character: 0,
            },
            end: lsp_types::Position {
                line: range.end.line + 1,
                character: 0,
            },
        },
        new_text: "# cmakefmt: on\n".to_string(),
    };

    // Uri has interior mutability; suppress the clippy lint (same as main_loop).
    #[allow(clippy::mutable_key_type)]
    let mut changes = std::collections::HashMap::new();
    changes.insert(uri, vec![off_edit, on_edit]);

    let action = lsp_types::CodeAction {
        title: "Disable cmakefmt for selection".to_string(),
        kind: Some(lsp_types::CodeActionKind::QUICKFIX),
        edit: Some(lsp_types::WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        ..Default::default()
    };

    let actions = vec![lsp_types::CodeActionOrCommand::CodeAction(action)];
    let result = match serde_json::to_value(actions) {
        Ok(v) => v,
        Err(err) => {
            return Some(Response::new_err(
                id,
                lsp_server::ErrorCode::InternalError as i32,
                format!("failed to serialize code action response: {err}"),
            ));
        }
    };
    Some(Response::new_ok(id, result))
}

/// Build a [`lsp_types::TextEdit`] that replaces the entire document.
fn full_document_edit(original: &str, formatted: String) -> lsp_types::TextEdit {
    let lines: Vec<&str> = original.lines().collect();
    let last_line = lines.len().saturating_sub(1);
    // LSP `Position.character` is measured in UTF-16 code units by default, so
    // count UTF-16 units rather than UTF-8 bytes — otherwise a non-ASCII last
    // line produces an end column that overshoots the real text.
    let last_char = lines
        .last()
        .map(|l: &&str| l.encode_utf16().count())
        .unwrap_or(0) as u32;
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

    #[test]
    fn full_document_edit_counts_utf16_units() {
        let original = "message(cafe)\nmessage(\"é\")\n";
        let edit = full_document_edit(original, original.to_string());
        assert_eq!(edit.range.end.line, 1);
        assert_eq!(
            edit.range.end.character,
            "message(\"é\")".encode_utf16().count() as u32
        );
    }

    // ── handle_formatting ─────────────────────────────────────────────────

    #[test]
    fn handle_formatting_returns_formatted_edit() {
        let uri = "file:///test.cmake";
        let text = "MESSAGE(hello)\n";
        let resp = handle_formatting(
            formatting_request(uri),
            &docs(uri, text),
            &Config::default(),
        )
        .unwrap();
        assert!(resp.error.is_none());
        let edits: Vec<lsp_types::TextEdit> = serde_json::from_value(resp.result.unwrap()).unwrap();
        assert_eq!(edits.len(), 1);
        // Default config lowercases commands
        assert!(edits[0].new_text.starts_with("message("));
    }

    #[test]
    fn handle_formatting_returns_none_for_unknown_uri() {
        let resp = handle_formatting(
            formatting_request("file:///missing.cmake"),
            &HashMap::new(),
            &Config::default(),
        );
        assert!(resp.is_none());
    }

    // ── handle_range_formatting ───────────────────────────────────────────

    #[test]
    fn handle_range_formatting_formats_selected_lines() {
        let uri = "file:///test.cmake";
        // Three-line document; format only line 1 (0-based)
        let text = "message(a)\nMESSAGE(b)\nmessage(c)\n";
        let resp = handle_range_formatting(
            range_formatting_request(uri, 1, 1),
            &docs(uri, text),
            &Config::default(),
        )
        .unwrap();
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
            &Config::default(),
        );
        assert!(resp.is_none());
    }

    #[test]
    fn handle_range_formatting_ignores_range_start_past_eof() {
        let uri = "file:///test.cmake";
        let resp = handle_range_formatting(
            range_formatting_request(uri, 5, 7),
            &docs(uri, "message(a)\n"),
            &Config::default(),
        )
        .unwrap();
        assert!(resp.error.is_none());
        let edits: Vec<lsp_types::TextEdit> = serde_json::from_value(resp.result.unwrap()).unwrap();
        assert!(edits.is_empty());
    }

    #[test]
    fn handle_range_formatting_counts_utf16_units() {
        let uri = "file:///test.cmake";
        let text = "message(a)\nMESSAGE(\"é\")\n";
        let resp = handle_range_formatting(
            range_formatting_request(uri, 1, 1),
            &docs(uri, text),
            &Config::default(),
        )
        .unwrap();
        assert!(resp.error.is_none());
        let edits: Vec<lsp_types::TextEdit> = serde_json::from_value(resp.result.unwrap()).unwrap();
        assert_eq!(
            edits[0].range.end.character,
            "MESSAGE(\"é\")".encode_utf16().count() as u32
        );
    }

    // ── handle_request routing ────────────────────────────────────────────

    #[test]
    fn handle_request_returns_method_not_found_for_unknown_method() {
        let req = Request {
            id: RequestId::from(99),
            method: "unknown/method".to_string(),
            params: serde_json::Value::Null,
        };
        let resp = handle_request(req, &HashMap::new(), &Config::default()).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(
            resp.error.unwrap().code,
            lsp_server::ErrorCode::MethodNotFound as i32
        );
    }

    // ── handle_code_action ────────────────────────────────────────────────

    #[test]
    fn handle_code_action_returns_disable_action() {
        let params = lsp_types::CodeActionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: "file:///test.cmake".parse().unwrap(),
            },
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 2,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: 4,
                    character: 0,
                },
            },
            context: lsp_types::CodeActionContext::default(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let req = Request {
            id: RequestId::from(3),
            method: lsp_types::request::CodeActionRequest::METHOD.to_string(),
            params: serde_json::to_value(params).unwrap(),
        };
        let resp = handle_code_action(req).unwrap();
        assert!(resp.error.is_none());
        let actions: Vec<lsp_types::CodeActionOrCommand> =
            serde_json::from_value(resp.result.unwrap()).unwrap();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            lsp_types::CodeActionOrCommand::CodeAction(action) => {
                assert!(action.title.contains("Disable"));
            }
            _ => panic!("expected CodeAction"),
        }
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
        handle_notification(notif, &mut docs, &mut Config::default());
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
        handle_notification(notif, &mut docs, &mut Config::default());
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
        handle_notification(notif, &mut docs, &mut Config::default());
        assert!(!docs.contains_key(uri));
    }

    #[test]
    fn handle_notification_ignores_unknown_method() {
        let mut docs = HashMap::new();
        let notif = Notification {
            method: "unknown/notification".to_string(),
            params: serde_json::Value::Null,
        };
        handle_notification(notif, &mut docs, &mut Config::default()); // should not panic
    }

    // ── per-document config (uri_to_path / percent_decode / config_for_document) ──

    #[test]
    fn uri_to_path_decodes_file_uri() {
        let path = uri_to_path("file:///home/user/CMakeLists.txt").unwrap();
        assert_eq!(path, PathBuf::from("/home/user/CMakeLists.txt"));
    }

    #[test]
    fn uri_to_path_percent_decodes_spaces() {
        let path = uri_to_path("file:///home/my%20project/CMakeLists.txt").unwrap();
        assert_eq!(path, PathBuf::from("/home/my project/CMakeLists.txt"));
    }

    #[test]
    fn uri_to_path_rejects_non_file_uri() {
        // `untitled:` (unsaved buffer) and other non-`file:` schemes have no
        // filesystem path.
        assert!(uri_to_path("untitled:Untitled-1").is_none());
    }

    #[test]
    fn percent_decode_passes_through_invalid_escape() {
        // A `%` not followed by two hex digits is left verbatim rather than
        // dropped or mangled.
        assert_eq!(percent_decode("100%done"), "100%done");
        assert_eq!(percent_decode("a%2zb"), "a%2zb");
    }

    #[test]
    fn config_for_document_falls_back_for_unsaved_document() {
        // A non-`file:` URI has no on-disk location, so the fallback
        // (workspace/default) config is returned unchanged.
        let fallback = Config {
            line_width: 42,
            ..Config::default()
        };
        let config = config_for_document("untitled:Untitled-1", &fallback);
        assert_eq!(config.line_width, 42);
    }

    #[test]
    fn config_for_document_uses_nearest_project_config() {
        // A saved document discovers `.cmakefmt.*` next to it, overriding the
        // fallback config.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".cmakefmt.toml"),
            "[format]\nline_width = 40\n",
        )
        .unwrap();
        let file = dir.path().join("CMakeLists.txt");
        std::fs::write(&file, "message(hi)\n").unwrap();
        let uri = format!("file://{}", file.display());

        let fallback = Config {
            line_width: 100,
            ..Config::default()
        };
        let config = config_for_document(&uri, &fallback);
        assert_eq!(config.line_width, 40);
    }
}
