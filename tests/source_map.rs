//! GBS-022: `--debug-info` writes a source map (block id -> .gs file:line:column).
use std::{
    fs,
    io::Read,
};

use libgoboscript::{
    codegen::build::BuildOptions,
    frontend::build::build,
};
use serde_json::Value;

const MAIN: &str = "costumes \"blank.svg\";
%include lib/twice
%include std/answer

onflag {
    say 1;
    twice;
    say ANSWER + 1;
    say f(5);
}
";

const LIB: &str = "proc twice {
    say 2;
    say 3;
}
func f(x) {
    if $x > 1 { return 1; }
    return 2;
}
";

fn project_json(sb3: &[u8]) -> Value {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(sb3)).unwrap();
    let mut s = String::new();
    zip.by_name("project.json")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

#[test]
fn debug_info_maps_blocks_to_source_lines() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("proj");
    let std_dir = dir.path().join("std");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&std_dir).unwrap();
    fs::write(std_dir.join("answer.gs"), "%define ANSWER 41\n").unwrap();
    fs::write(root.join("blank.svg"), "<svg xmlns=\"http://www.w3.org/2000/svg\"/>").unwrap();
    fs::write(root.join("stage.gs"), "costumes \"blank.svg\";").unwrap();
    fs::write(root.join("main.gs"), MAIN).unwrap();
    fs::create_dir_all(root.join("lib")).unwrap();
    fs::write(root.join("lib/twice.gs"), LIB).unwrap();
    let out = dir.path().join("out.sb3");
    let map_path = dir.path().join("out.map.json");
    let artifact = build(
        Some(root),
        Some(out.clone()),
        &BuildOptions {
            std_path: Some(std_dir),
            debug_info: Some(map_path.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!artifact.failure());
    let map: Value = serde_json::from_slice(&fs::read(&map_path).unwrap()).unwrap();
    let project = project_json(&fs::read(&out).unwrap());
    let main = project["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "main")
        .unwrap();
    let blocks = main["blocks"].as_object().unwrap();
    let entries = map["targets"]["main"]["blocks"].as_object().unwrap();
    let location = |opcode: &str, message: Option<i64>| -> (String, u64) {
        let (id, _) = blocks
            .iter()
            .find(|(_, b)| {
                b["opcode"] == opcode
                    && message.is_none_or(|m| b["inputs"]["MESSAGE"][1][1] == m)
            })
            .unwrap_or_else(|| panic!("no {opcode} {message:?}"));
        let entry = &entries[id.as_str()];
        (
            entry["file"].as_str().unwrap().to_owned(),
            entry["line"].as_u64().unwrap(),
        )
    };
    assert_eq!(location("event_whenflagclicked", None), ("main.gs".into(), 5));
    assert_eq!(location("looks_say", Some(1)), ("main.gs".into(), 6));
    assert_eq!(location("procedures_call", None), ("main.gs".into(), 7));
    assert_eq!(location("looks_say", Some(42)), ("main.gs".into(), 8));
    assert_eq!(location("procedures_definition", None), ("lib/twice.gs".into(), 1));
    assert_eq!(location("looks_say", Some(2)), ("lib/twice.gs".into(), 2));
    assert_eq!(location("looks_say", Some(3)), ("lib/twice.gs".into(), 3));
    // blocks generated for `return` map to the return statement
    assert_eq!(location("control_stop", None), ("lib/twice.gs".into(), 6));
    // every non-shadow block has an entry
    for (id, block) in blocks {
        if block["shadow"] == true || block["opcode"] == "procedures_prototype" {
            continue;
        }
        assert!(entries.contains_key(id.as_str()), "{id} {block} not mapped");
    }
}
