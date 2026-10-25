//! GBS-024: `--std-path` uses a local standard library without network access.
mod common;

use std::fs;

use common::*;
use libgoboscript::codegen::build::BuildOptions;

#[test]
fn std_path_is_used_without_network() {
    let std_dir = tempfile::tempdir().unwrap();
    fs::write(std_dir.path().join("answer.gs"), "%define ANSWER 42\n").unwrap();
    // no `std = ...` in goboscript.toml: without --std-path this would ask the GitHub API
    let b = build_with(
        &[(
            "main.gs",
            "%include std/answer\ncostumes \"blank.svg\";\nonflag { say ANSWER; }",
        )],
        &BuildOptions {
            std_path: Some(std_dir.path().to_path_buf()),
            ..Default::default()
        },
    );
    assert!(!b.failed(), "{:?}", b.diagnostics());
    assert_eq!(
        input(&b, "looks_say", "MESSAGE"),
        serde_json::json!([1, [4, 42]])
    );
}

#[test]
fn missing_std_path_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("p");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("stage.gs"), "costumes \"blank.svg\";").unwrap();
    fs::write(root.join("blank.svg"), BLANK_SVG).unwrap();
    let result = std::panic::catch_unwind(|| {
        build_in(
            tempfile::tempdir().unwrap(),
            root.clone(),
            root.clone(),
            &BuildOptions {
                std_path: Some(dir.path().join("does-not-exist")),
                ..Default::default()
            },
        )
    });
    assert!(result.is_err());
}
