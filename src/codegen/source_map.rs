//! Source map for a build (GBS-022): for every target, which `.gs` file, line and column each
//! emitted block comes from. Built from the per-sprite `DebugInfo` (block id -> span in the
//! translation unit) and the translation unit's include table.

use std::path::Path;

use rustc_hash::FxHashMap;
use serde_json::{
    json,
    Map,
    Value,
};

use crate::{
    diagnostic::{
        Artifact,
        SpriteDiagnostics,
    },
    translation_unit::Owner,
    vfs::VFS,
};

pub const SOURCE_MAP_VERSION: u64 = 1;

/// Line starts (byte offsets) and text of one source file, for offset -> line:column.
struct SourceFile {
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    fn new(text: String) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        Self { text, line_starts }
    }

    /// 1-based line and column (in characters) of a byte offset.
    fn line_column(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.text.len());
        let line = self.line_starts.partition_point(|start| *start <= offset) - 1;
        let start = self.line_starts[line];
        let column = self
            .text
            .get(start..offset)
            .map_or(offset - start, |prefix| prefix.chars().count());
        (line + 1, column + 1)
    }
}

/// Maps translation-unit spans to `.gs` file locations (shared by the source map and the AST
/// dump).
pub struct Locator<'a> {
    input: &'a Path,
    std_path: Option<&'a Path>,
    files: FxHashMap<std::path::PathBuf, Option<SourceFile>>,
}

impl<'a> Locator<'a> {
    pub fn new(input: &'a Path, std_path: Option<&'a Path>) -> Self {
        Self {
            input,
            std_path,
            files: Default::default(),
        }
    }

    /// `{file, owner, line, column, endLine, endColumn}` of the span `start..end` of `unit`
    /// (1-based lines and columns, end column exclusive), or None for generated spans (0..0)
    /// and spans outside the unit.
    pub fn locate(
        &mut self,
        unit: &crate::translation_unit::TranslationUnit,
        span_start: usize,
        span_end: usize,
        fs: &mut dyn VFS,
    ) -> Option<Value> {
        if span_start == 0 && span_end == 0 || span_start >= unit.text.len() {
            return None;
        }
        let (start, include) = unit.translate_position(span_start);
        let end_position = span_end.max(span_start + 1) - 1;
        let end = if include.unit_range.contains(&end_position) {
            start + (end_position - span_start)
        } else {
            start
        };
        let file = self
            .files
            .entry(include.path.clone())
            .or_insert_with(|| fs.read_to_string(&include.path).ok().map(SourceFile::new));
        let file = file.as_ref()?;
        let (line, column) = file.line_column(start);
        let (end_line, end_column) = file.line_column(end);
        let owner = match include.owner {
            Owner::Local => "local",
            Owner::StandardLibrary => "std",
        };
        let path = match include.owner {
            Owner::Local => include
                .path
                .strip_prefix(self.input)
                .unwrap_or(&include.path),
            Owner::StandardLibrary => self
                .std_path
                .and_then(|std_path| include.path.strip_prefix(std_path).ok())
                .or_else(|| include.path.file_name().map(Path::new))
                .unwrap_or(&include.path),
        };
        Some(json!({
            "file": path.to_string_lossy().replace('\\', "/"),
            "owner": owner,
            "line": line,
            "column": column,
            "endLine": end_line,
            "endColumn": end_column + 1,
        }))
    }
}

/// Targets of `artifact` in source map order: the Stage first, then sprites by name.
pub fn targets_of(artifact: &Artifact) -> Vec<(&str, &SpriteDiagnostics)> {
    let mut sprites: Vec<(&str, &SpriteDiagnostics)> = vec![("Stage", &artifact.stage_diagnostics)];
    let mut names: Vec<_> = artifact.sprites_diagnostics.keys().collect();
    names.sort();
    for name in names {
        sprites.push((name.as_str(), &artifact.sprites_diagnostics[name]));
    }
    sprites
}

/// Build the source map of `artifact`. File paths are relative to the project directory
/// `input`; standard library files are marked `"owner": "std"` and are relative to `std_path`
/// when given (otherwise only the file name is kept). Generated blocks without a source
/// location are omitted.
pub fn source_map(
    artifact: &Artifact,
    input: &Path,
    std_path: Option<&Path>,
    fs: &mut dyn VFS,
) -> Value {
    let mut locator = Locator::new(input, std_path);
    let mut targets = Map::new();
    for (target_name, diagnostics) in targets_of(artifact) {
        let unit = &diagnostics.translation_unit;
        let mut ids: Vec<_> = diagnostics.debug_info.blocks.iter().collect();
        ids.sort_by(|a, b| a.0.cmp(b.0));
        let mut blocks = Map::new();
        for (id, span) in ids {
            if let Some(location) = locator.locate(unit, span.start, span.end, fs) {
                blocks.insert(id.trim_matches('"').to_string(), location);
            }
        }
        targets.insert(target_name.to_string(), json!({ "blocks": blocks }));
    }
    json!({ "version": SOURCE_MAP_VERSION, "targets": targets })
}
