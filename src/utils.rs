//! Utility functions and helpers for LSP Bridge.

use crate::error::{LspError, Result};
use lsp_types::*;
use std::path::{Path, PathBuf};
use url::Url;

/// Utility functions for working with URIs.
pub mod uri {
    use super::*;

    /// Convert a file path to an LSP URI.
    ///
    /// Relative paths are made absolute first. The result is percent-encoded
    /// (`file:///C:/dir/a%20b` on Windows, `file:///dir/a%20b` elsewhere).
    pub fn from_path<P: AsRef<Path>>(path: P) -> String {
        let path = path.as_ref();
        let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());

        match Url::from_file_path(&absolute) {
            Ok(url) => url.to_string(),
            // Not representable as a file URL (e.g. a Windows path with no drive or UNC prefix):
            // keep the unescaped fallback rather than fail.
            Err(()) => format!("file://{}", path.to_string_lossy().replace('\\', "/")),
        }
    }

    /// Convert an LSP URI to a file path.
    pub fn to_path(uri: &str) -> Result<PathBuf> {
        let url = Url::parse(uri).map_err(|_| LspError::invalid_uri(uri))?;

        if url.scheme() != "file" {
            return Err(LspError::invalid_uri(uri).into());
        }

        url.to_file_path()
            .map_err(|_| LspError::invalid_uri(uri).into())
    }

    /// Normalize a URI by ensuring it's properly formatted.
    pub fn normalize(uri: &str) -> Result<String> {
        let url = Url::parse(uri).map_err(|_| LspError::invalid_uri(uri))?;
        Ok(url.to_string())
    }

    /// Check if a URI is valid.
    pub fn is_valid(uri: &str) -> bool {
        Url::parse(uri).is_ok()
    }

    /// Get the file name from a URI.
    pub fn file_name(uri: &str) -> Option<String> {
        to_path(uri).ok().and_then(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|s| s.to_string())
        })
    }

    /// Get the directory from a URI.
    pub fn directory(uri: &str) -> Option<String> {
        to_path(uri)
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
            .map(from_path)
    }

    /// Join a base URI with a relative path.
    pub fn join(base: &str, relative: &str) -> Result<String> {
        let base_url = Url::parse(base).map_err(|_| LspError::invalid_uri(base))?;

        let joined = base_url
            .join(relative)
            .map_err(|_| LspError::invalid_uri(relative))?;

        Ok(joined.to_string())
    }
}

/// Utility functions for working with positions and ranges.
pub mod position {
    use super::*;

    /// Create a new position.
    pub fn new(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    /// Create a position at the start of a line.
    pub fn line_start(line: u32) -> Position {
        Position { line, character: 0 }
    }

    /// Create a position at the end of a line.
    pub fn line_end(line: u32, line_length: u32) -> Position {
        Position {
            line,
            character: line_length,
        }
    }

    /// Check if a position is before another position.
    pub fn is_before(pos1: &Position, pos2: &Position) -> bool {
        pos1.line < pos2.line || (pos1.line == pos2.line && pos1.character < pos2.character)
    }

    /// Check if a position is after another position.
    pub fn is_after(pos1: &Position, pos2: &Position) -> bool {
        pos1.line > pos2.line || (pos1.line == pos2.line && pos1.character > pos2.character)
    }

    /// Get the distance between two positions (in characters).
    pub fn distance(start: &Position, end: &Position, line_lengths: &[u32]) -> u32 {
        if start.line == end.line {
            end.character.saturating_sub(start.character)
        } else {
            let mut total = 0;

            // Characters from start to end of start line
            if (start.line as usize) < line_lengths.len() {
                total += line_lengths[start.line as usize].saturating_sub(start.character);
            }

            // Full lines in between
            for line in (start.line + 1)..end.line {
                if (line as usize) < line_lengths.len() {
                    total += line_lengths[line as usize] + 1; // +1 for newline
                }
            }

            // Characters from start of end line to end position
            total += end.character;

            total
        }
    }
}

/// Utility functions for working with ranges.
pub mod range {
    use super::*;

    /// Create a new range.
    pub fn new(start: Position, end: Position) -> Range {
        Range { start, end }
    }

    /// Create a range from coordinates.
    pub fn from_coords(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> Range {
        Range {
            start: Position {
                line: start_line,
                character: start_char,
            },
            end: Position {
                line: end_line,
                character: end_char,
            },
        }
    }

    /// Create a range that covers a single line.
    pub fn line(line: u32, line_length: u32) -> Range {
        Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: line_length,
            },
        }
    }

    /// Create a range for a single character.
    pub fn character(line: u32, character: u32) -> Range {
        Range {
            start: Position { line, character },
            end: Position {
                line,
                character: character + 1,
            },
        }
    }

    /// Check if a range is empty (start == end).
    pub fn is_empty(range: &Range) -> bool {
        range.start == range.end
    }

    /// Check if a position is within a range.
    pub fn contains_position(range: &Range, position: &Position) -> bool {
        (position.line > range.start.line
            || (position.line == range.start.line && position.character >= range.start.character))
            && (position.line < range.end.line
                || (position.line == range.end.line && position.character <= range.end.character))
    }

    /// Check if two ranges overlap.
    pub fn overlaps(range1: &Range, range2: &Range) -> bool {
        !is_before(range1, range2) && !is_after(range1, range2)
    }

    /// Check if range1 is before range2.
    pub fn is_before(range1: &Range, range2: &Range) -> bool {
        position::is_before(&range1.end, &range2.start)
    }

    /// Check if range1 is after range2.
    pub fn is_after(range1: &Range, range2: &Range) -> bool {
        position::is_after(&range1.start, &range2.end)
    }

    /// Get the intersection of two ranges.
    pub fn intersection(range1: &Range, range2: &Range) -> Option<Range> {
        if !overlaps(range1, range2) {
            return None;
        }

        let start = if position::is_after(&range1.start, &range2.start) {
            range1.start
        } else {
            range2.start
        };

        let end = if position::is_before(&range1.end, &range2.end) {
            range1.end
        } else {
            range2.end
        };

        Some(Range { start, end })
    }

    /// Get the union of two ranges.
    pub fn union(range1: &Range, range2: &Range) -> Range {
        let start = if position::is_before(&range1.start, &range2.start) {
            range1.start
        } else {
            range2.start
        };

        let end = if position::is_after(&range1.end, &range2.end) {
            range1.end
        } else {
            range2.end
        };

        Range { start, end }
    }
}

/// Utility functions for working with text documents.
pub mod document {
    use super::*;

    /// Create a text document identifier.
    pub fn identifier(uri: Uri) -> TextDocumentIdentifier {
        TextDocumentIdentifier { uri }
    }

    /// Create a versioned text document identifier.
    pub fn versioned_identifier(uri: Uri, version: i32) -> VersionedTextDocumentIdentifier {
        VersionedTextDocumentIdentifier { uri, version }
    }

    /// Create a text document item.
    pub fn item<L, T>(uri: Uri, language_id: L, version: i32, text: T) -> TextDocumentItem
    where
        L: Into<String>,
        T: Into<String>,
    {
        TextDocumentItem {
            uri,
            language_id: language_id.into(),
            version,
            text: text.into(),
        }
    }

    /// Create text document position parameters.
    pub fn position_params(uri: Uri, position: Position) -> TextDocumentPositionParams {
        TextDocumentPositionParams {
            text_document: identifier(uri),
            position,
        }
    }

    /// Get line lengths from text content.
    pub fn line_lengths(content: &str) -> Vec<u32> {
        content.lines().map(|line| line.len() as u32).collect()
    }

    /// Get the line at a specific line number.
    pub fn get_line(content: &str, line_number: u32) -> Option<&str> {
        content.lines().nth(line_number as usize)
    }

    /// Get text within a range.
    pub fn get_range_text(content: &str, range: &Range) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();

        if range.start.line as usize >= lines.len() || range.end.line as usize >= lines.len() {
            return Err(LspError::custom("Range out of bounds").into());
        }

        if range.start.line == range.end.line {
            let line = lines[range.start.line as usize];
            let start_char = range.start.character as usize;
            let end_char = range.end.character as usize;

            if start_char > line.len() || end_char > line.len() {
                return Err(LspError::custom("Character position out of bounds").into());
            }

            return Ok(line[start_char..end_char].to_string());
        }

        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            let line_num = i as u32;
            if line_num >= range.start.line && line_num <= range.end.line {
                if line_num == range.start.line {
                    let start_char = range.start.character as usize;
                    if start_char <= line.len() {
                        result.push_str(&line[start_char..]);
                    }
                } else if line_num == range.end.line {
                    let end_char = range.end.character as usize;
                    if end_char <= line.len() {
                        result.push_str(&line[..end_char]);
                    }
                } else {
                    result.push_str(line);
                }

                if line_num < range.end.line {
                    result.push('\n');
                }
            }
        }

        Ok(result)
    }

    /// Apply a text edit to content.
    pub fn apply_edit(content: &str, edit: &TextEdit) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result_lines = Vec::new();

        let start_line = edit.range.start.line as usize;
        let start_char = edit.range.start.character as usize;
        let end_line = edit.range.end.line as usize;
        let end_char = edit.range.end.character as usize;

        if start_line >= lines.len() || end_line >= lines.len() {
            return Err(LspError::custom("Edit range out of bounds").into());
        }

        // Copy lines before the edit
        for line in lines.iter().take(start_line) {
            result_lines.push(line.to_string());
        }

        // Apply the edit
        if start_line == end_line {
            // Single line edit
            let line = lines[start_line];
            if start_char <= line.len() && end_char <= line.len() {
                let mut new_line = String::new();
                new_line.push_str(&line[..start_char]);
                new_line.push_str(&edit.new_text);
                new_line.push_str(&line[end_char..]);
                result_lines.push(new_line);
            } else {
                return Err(LspError::custom("Edit character positions out of bounds").into());
            }
        } else {
            // Multi-line edit
            let start_line_content = lines[start_line];
            let end_line_content = lines[end_line];

            let mut new_content = String::new();
            new_content.push_str(&start_line_content[..start_char.min(start_line_content.len())]);
            new_content.push_str(&edit.new_text);
            new_content.push_str(&end_line_content[end_char.min(end_line_content.len())..]);

            // Add the new content as separate lines
            for line in new_content.lines() {
                result_lines.push(line.to_string());
            }
        }

        // Copy lines after the edit
        for line in lines.iter().skip(end_line + 1) {
            result_lines.push(line.to_string());
        }

        Ok(result_lines.join("\n"))
    }

    /// Apply multiple text edits to content.
    pub fn apply_edits(content: &str, edits: &[TextEdit]) -> Result<String> {
        let mut result = content.to_string();

        // Sort edits by position (reverse order to apply from end to beginning)
        let mut sorted_edits = edits.to_vec();
        sorted_edits.sort_by(|a, b| {
            b.range
                .start
                .line
                .cmp(&a.range.start.line)
                .then_with(|| b.range.start.character.cmp(&a.range.start.character))
        });

        for edit in sorted_edits {
            result = apply_edit(&result, &edit)?;
        }

        Ok(result)
    }
}

/// Utility functions for working with completion items.
pub mod completion {
    use super::*;

    /// Create a simple completion item.
    pub fn item<L: Into<String>>(label: L, kind: Option<CompletionItemKind>) -> CompletionItem {
        CompletionItem {
            label: label.into(),
            kind,
            ..Default::default()
        }
    }

    /// Create a completion item with detail.
    pub fn item_with_detail<L, D>(
        label: L,
        detail: D,
        kind: Option<CompletionItemKind>,
    ) -> CompletionItem
    where
        L: Into<String>,
        D: Into<String>,
    {
        CompletionItem {
            label: label.into(),
            detail: Some(detail.into()),
            kind,
            ..Default::default()
        }
    }

    /// Sort completion items by relevance.
    pub fn sort_by_relevance(items: &mut [CompletionItem]) {
        items.sort_by(|a, b| {
            // Sort by sort_text if available, otherwise by label
            let a_sort = a.sort_text.as_ref().unwrap_or(&a.label);
            let b_sort = b.sort_text.as_ref().unwrap_or(&b.label);
            a_sort.cmp(b_sort)
        });
    }

    /// Filter completion items by prefix.
    pub fn filter_by_prefix(items: &[CompletionItem], prefix: &str) -> Vec<CompletionItem> {
        let prefix_lower = prefix.to_lowercase();
        items
            .iter()
            .filter(|item| {
                item.label.to_lowercase().starts_with(&prefix_lower)
                    || item
                        .filter_text
                        .as_ref()
                        .map(|text| text.to_lowercase().starts_with(&prefix_lower))
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }
}

/// Utility functions for working with diagnostics.
pub mod diagnostics {
    use super::*;

    /// Create a diagnostic.
    pub fn create<M: Into<String>>(
        range: Range,
        severity: Option<DiagnosticSeverity>,
        message: M,
    ) -> Diagnostic {
        Diagnostic {
            range,
            severity,
            code: None,
            code_description: None,
            source: None,
            message: message.into(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    /// Create an error diagnostic.
    pub fn error<M: Into<String>>(range: Range, message: M) -> Diagnostic {
        create(range, Some(DiagnosticSeverity::ERROR), message)
    }

    /// Create a warning diagnostic.
    pub fn warning<M: Into<String>>(range: Range, message: M) -> Diagnostic {
        create(range, Some(DiagnosticSeverity::WARNING), message)
    }

    /// Create an info diagnostic.
    pub fn info<M: Into<String>>(range: Range, message: M) -> Diagnostic {
        create(range, Some(DiagnosticSeverity::INFORMATION), message)
    }

    /// Create a hint diagnostic.
    pub fn hint<M: Into<String>>(range: Range, message: M) -> Diagnostic {
        create(range, Some(DiagnosticSeverity::HINT), message)
    }

    /// Filter diagnostics by severity.
    pub fn filter_by_severity(
        diagnostics: &[Diagnostic],
        severity: DiagnosticSeverity,
    ) -> Vec<Diagnostic> {
        diagnostics
            .iter()
            .filter(|diag| diag.severity == Some(severity))
            .cloned()
            .collect()
    }

    /// Sort diagnostics by position.
    pub fn sort_by_position(diagnostics: &mut [Diagnostic]) {
        diagnostics.sort_by(|a, b| {
            a.range
                .start
                .line
                .cmp(&b.range.start.line)
                .then_with(|| a.range.start.character.cmp(&b.range.start.character))
        });
    }
}

/// Utility functions for working with language server features.
pub mod features {
    use super::*;

    /// Check if a server supports a specific capability.
    pub fn supports_capability(capabilities: &ServerCapabilities, feature: &str) -> bool {
        match feature {
            "textDocumentSync" => capabilities.text_document_sync.is_some(),
            "completion" => capabilities.completion_provider.is_some(),
            "hover" => capabilities.hover_provider.is_some(),
            "signatureHelp" => capabilities.signature_help_provider.is_some(),
            "definition" => capabilities.definition_provider.is_some(),
            "typeDefinition" => capabilities.type_definition_provider.is_some(),
            "implementation" => capabilities.implementation_provider.is_some(),
            "references" => capabilities.references_provider.is_some(),
            "documentHighlight" => capabilities.document_highlight_provider.is_some(),
            "documentSymbol" => capabilities.document_symbol_provider.is_some(),
            "codeAction" => capabilities.code_action_provider.is_some(),
            "codeLens" => capabilities.code_lens_provider.is_some(),
            "documentLink" => capabilities.document_link_provider.is_some(),
            "colorProvider" => capabilities.color_provider.is_some(),
            "formatting" => capabilities.document_formatting_provider.is_some(),
            "rangeFormatting" => capabilities.document_range_formatting_provider.is_some(),
            "onTypeFormatting" => capabilities.document_on_type_formatting_provider.is_some(),
            "rename" => capabilities.rename_provider.is_some(),
            "foldingRange" => capabilities.folding_range_provider.is_some(),
            "executeCommand" => capabilities.execute_command_provider.is_some(),
            "selectionRange" => capabilities.selection_range_provider.is_some(),
            "workspaceSymbol" => capabilities.workspace_symbol_provider.is_some(),
            _ => false,
        }
    }

    /// Get a list of supported features from server capabilities.
    pub fn list_supported_features(capabilities: &ServerCapabilities) -> Vec<String> {
        let mut features = Vec::new();

        if capabilities.text_document_sync.is_some() {
            features.push("textDocumentSync".to_string());
        }
        if capabilities.completion_provider.is_some() {
            features.push("completion".to_string());
        }
        if capabilities.hover_provider.is_some() {
            features.push("hover".to_string());
        }
        if capabilities.signature_help_provider.is_some() {
            features.push("signatureHelp".to_string());
        }
        if capabilities.definition_provider.is_some() {
            features.push("definition".to_string());
        }
        if capabilities.type_definition_provider.is_some() {
            features.push("typeDefinition".to_string());
        }
        if capabilities.implementation_provider.is_some() {
            features.push("implementation".to_string());
        }
        if capabilities.references_provider.is_some() {
            features.push("references".to_string());
        }
        if capabilities.document_highlight_provider.is_some() {
            features.push("documentHighlight".to_string());
        }
        if capabilities.document_symbol_provider.is_some() {
            features.push("documentSymbol".to_string());
        }
        if capabilities.code_action_provider.is_some() {
            features.push("codeAction".to_string());
        }
        if capabilities.code_lens_provider.is_some() {
            features.push("codeLens".to_string());
        }
        if capabilities.document_link_provider.is_some() {
            features.push("documentLink".to_string());
        }
        if capabilities.color_provider.is_some() {
            features.push("colorProvider".to_string());
        }
        if capabilities.document_formatting_provider.is_some() {
            features.push("formatting".to_string());
        }
        if capabilities.document_range_formatting_provider.is_some() {
            features.push("rangeFormatting".to_string());
        }
        if capabilities.document_on_type_formatting_provider.is_some() {
            features.push("onTypeFormatting".to_string());
        }
        if capabilities.rename_provider.is_some() {
            features.push("rename".to_string());
        }
        if capabilities.folding_range_provider.is_some() {
            features.push("foldingRange".to_string());
        }
        if capabilities.execute_command_provider.is_some() {
            features.push("executeCommand".to_string());
        }
        if capabilities.selection_range_provider.is_some() {
            features.push("selectionRange".to_string());
        }
        if capabilities.workspace_symbol_provider.is_some() {
            features.push("workspaceSymbol".to_string());
        }

        features
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_conversion() {
        // Use a platform-appropriate path
        let path = if cfg!(windows) {
            Path::new("C:\\tmp\\test.rs")
        } else {
            Path::new("/tmp/test.rs")
        };

        let uri = uri::from_path(path);
        let back_path = uri::to_path(&uri).unwrap();
        assert_eq!(back_path, path);
    }

    #[test]
    fn test_position_utilities() {
        let pos1 = position::new(1, 5);
        let pos2 = position::new(2, 3);

        assert!(position::is_before(&pos1, &pos2));
        assert!(!position::is_after(&pos1, &pos2));
    }

    #[test]
    fn test_range_utilities() {
        let range1 = range::from_coords(1, 0, 1, 10);
        let range2 = range::from_coords(1, 5, 1, 15);

        assert!(range::overlaps(&range1, &range2));

        let intersection = range::intersection(&range1, &range2).unwrap();
        assert_eq!(intersection.start.character, 5);
        assert_eq!(intersection.end.character, 10);
    }

    #[test]
    fn test_document_utilities() {
        let content = "line 1\nline 2\nline 3";
        let range = range::from_coords(1, 0, 1, 6);

        let range_text = document::get_range_text(content, &range).unwrap();
        assert_eq!(range_text, "line 2");
    }

    #[test]
    fn test_text_edit_application() {
        let content = "hello world";
        let edit = TextEdit {
            range: range::from_coords(0, 6, 0, 11),
            new_text: "LSP".to_string(),
        };

        let result = document::apply_edit(content, &edit).unwrap();
        assert_eq!(result, "hello LSP");
    }
}
