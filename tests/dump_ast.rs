//! `--dump-ast`: the parsed AST before any pass, with source locations for every span.
use std::fs;

use libgoboscript::{
    codegen::build::BuildOptions,
    frontend::build::build,
};
use serde_json::Value;

const MAIN: &str = "costumes \"blank.svg\";
var a = 0;
proc unused n {
    a = $n + 2.50 * -3;
    a = if (a > 1) 5 else 6;
}
onflag { say 1 <= 2; }
";

fn dump(no_fold: bool) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("p");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("main.gs"), MAIN).unwrap();
    fs::write(root.join("stage.gs"), "costumes \"blank.svg\";\n").unwrap();
    fs::write(
        root.join("blank.svg"),
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2"></svg>"#,
    )
    .unwrap();
    let std_path = dir.path().join("std");
    fs::create_dir_all(&std_path).unwrap();
    let out = dir.path().join("out.sb3");
    let ast = dir.path().join("ast.json");
    let artifact = build(
        Some(root),
        Some(out),
        &BuildOptions {
            std_path: Some(std_path),
            dump_ast: Some(ast.clone()),
            no_fold,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!artifact.failure());
    serde_json::from_slice(&fs::read(ast).unwrap()).unwrap()
}

#[test]
fn dump_is_the_parsed_ast_with_locations() {
    for no_fold in [false, true] {
        let d = dump(no_fold);
        assert_eq!(d["version"], 1);
        let main = &d["targets"]["main"];
        assert!(d["targets"]["Stage"].is_object());
        // the unused procedure is in the dump even when dead-code removal drops it
        let body = main["proc_definitions"]["unused"].as_array().unwrap();
        let value = &body[0]["SetVar"]["value"]["BinOp"];
        assert_eq!(value["op"], "Add");
        // not folded, not lowered: 2.50 * -3 is still a Mul of a literal and a Minus
        let mul = &value["rhs"]["BinOp"];
        assert_eq!(mul["op"], "Mul");
        assert_eq!(mul["rhs"]["UnOp"]["op"], "Minus");
        let lit = &mul["lhs"]["Value"];
        assert_eq!(lit["value"], 2.5);
        let loc = &lit["span"]["loc"];
        assert_eq!(loc["file"], "main.gs");
        assert_eq!(loc["line"], 4);
        assert_eq!(loc["column"], 14);
        assert_eq!(loc["endColumn"], 18);
        // the ternary is still a ternary (the pass that extracts it runs later)
        assert!(body[1]["SetVar"]["value"]["Ternary"].is_object());
        // `<=` is not rewritten to `not >`
        let say = &main["events"][0]["body"][0]["Block"];
        assert_eq!(say["args"][0]["BinOp"]["op"], "Le");
    }
}
