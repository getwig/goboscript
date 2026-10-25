//! `--dump-ast PATH`: the parsed AST of every target, before any compiler pass (no ternary
//! extraction, no lowering, no folding, no dead-code removal), as JSON. Tools use it to
//! evaluate what the source means independently of the compiler, e.g. a reference evaluator
//! that checks the compiled blocks against the source.
//!
//! Format: `{"version": 1, "targets": {<target>: <Sprite AST>}}`. The Sprite AST is the serde
//! form of `ast::Sprite`. Every span object `{"start", "end"}` (byte offsets in the
//! preprocessed translation unit) additionally gets `"loc": {file, owner, line, column,
//! endLine, endColumn}` in the same form as the `--debug-info` source map, so AST nodes can be
//! matched with emitted blocks. Number literals are serialized as JSON numbers (integers up to
//! 2^53 as integers); boolean literals as the text "true"/"false" (as in inputs), so a
//! consumer that needs the literal kind or the exact token text reads it from the source at
//! `loc`.

use std::path::Path;

use serde_json::{
    json,
    Map,
    Value,
};

use super::source_map::{
    targets_of,
    Locator,
};
use crate::{
    diagnostic::Artifact,
    translation_unit::TranslationUnit,
    vfs::VFS,
};

pub const AST_DUMP_VERSION: u64 = 1;

/// Add `loc` to every span object below `value`.
fn locate_spans(
    value: &mut Value,
    unit: &TranslationUnit,
    locator: &mut Locator,
    fs: &mut dyn VFS,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                locate_spans(item, unit, locator, fs);
            }
        }
        Value::Object(map) => {
            let is_span = map.len() == 2
                && map.get("start").is_some_and(Value::is_u64)
                && map.get("end").is_some_and(Value::is_u64);
            if is_span {
                let start = map["start"].as_u64().unwrap() as usize;
                let end = map["end"].as_u64().unwrap() as usize;
                if let Some(location) = locator.locate(unit, start, end, fs) {
                    map.insert("loc".to_string(), location);
                }
                return;
            }
            for item in map.values_mut() {
                locate_spans(item, unit, locator, fs);
            }
        }
        _ => {}
    }
}

/// The dump of `artifact`, whose `parsed_ast` was captured right after parsing.
pub fn ast_dump(
    artifact: &Artifact,
    input: &Path,
    std_path: Option<&Path>,
    fs: &mut dyn VFS,
) -> Value {
    let mut locator = Locator::new(input, std_path);
    let mut targets = Map::new();
    let parsed = artifact.parsed_ast.as_ref();
    for (target_name, diagnostics) in targets_of(artifact) {
        let key = if target_name == "Stage" {
            "stage"
        } else {
            target_name
        };
        let Some(mut ast) = parsed.and_then(|parsed| parsed.get(key)).cloned() else {
            continue;
        };
        locate_spans(&mut ast, &diagnostics.translation_unit, &mut locator, fs);
        targets.insert(target_name.to_string(), ast);
    }
    json!({ "version": AST_DUMP_VERSION, "targets": targets })
}
