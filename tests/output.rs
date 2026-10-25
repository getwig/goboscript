//! GBS-008: a failed build leaves the output file alone; a successful one replaces it.
use std::fs;

use libgoboscript::{
    codegen::build::BuildOptions,
    frontend::build::build,
};

fn project(dir: &std::path::Path, main: &str) -> std::path::PathBuf {
    let root = dir.join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("blank.svg"), "<svg xmlns=\"http://www.w3.org/2000/svg\"/>").unwrap();
    fs::write(root.join("stage.gs"), "costumes \"blank.svg\";").unwrap();
    fs::write(root.join("main.gs"), main).unwrap();
    root
}

fn options(dir: &std::path::Path) -> BuildOptions {
    let std_dir = dir.join("std");
    fs::create_dir_all(&std_dir).unwrap();
    BuildOptions {
        std_path: Some(std_dir),
        ..Default::default()
    }
}

#[test]
fn failed_build_keeps_previous_output() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.sb3");
    fs::write(&out, b"previous build").unwrap();
    let root = project(dir.path(), "costumes \"blank.svg\";\nonflag { undefined_block; }");
    let artifact = build(Some(root), Some(out.clone()), &options(dir.path())).unwrap();
    assert!(artifact.failure());
    assert_eq!(fs::read(&out).unwrap(), b"previous build");
    let leftovers: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty());
}

#[test]
fn failed_build_creates_no_output() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.sb3");
    let root = project(dir.path(), "costumes \"blank.svg\";\nonflag { undefined_block; }");
    let artifact = build(Some(root), Some(out.clone()), &options(dir.path())).unwrap();
    assert!(artifact.failure());
    assert!(!out.exists());
}

#[test]
fn successful_build_replaces_output() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.sb3");
    fs::write(&out, b"previous build").unwrap();
    let root = project(dir.path(), "costumes \"blank.svg\";\nonflag { say 1; }");
    let artifact = build(Some(root), Some(out.clone()), &options(dir.path())).unwrap();
    assert!(!artifact.failure());
    let bytes = fs::read(&out).unwrap();
    assert_eq!(&bytes[..2], b"PK");
}
