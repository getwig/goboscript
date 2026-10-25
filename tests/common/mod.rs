//! Shared helpers for the compiler integration tests: build a small project from in-memory
//! sources in a temporary directory (offline, empty standard library) and inspect the result.
#![allow(dead_code)]

use std::{
    cell::RefCell,
    fs,
    io::{
        Cursor,
        Read,
    },
    path::PathBuf,
    rc::Rc,
};

use libgoboscript::{
    codegen::build::{
        build_impl,
        BuildOptions,
    },
    diagnostic::Artifact,
    standard_library::StandardLibrary,
    vfs::RealFS,
};
use semver::Version;
use serde_json::Value;

pub const BLANK_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2"></svg>"#;

pub struct Built {
    pub artifact: Artifact,
    pub sb3: Vec<u8>,
    pub project: Option<Value>,
    pub dir: tempfile::TempDir,
}

/// Build a project. `files` are (relative path, contents); `stage.gs` and `blank.svg` are added
/// when missing. The standard library is an empty local directory (no network).
pub fn build(files: &[(&str, &str)]) -> Built {
    build_with(files, &BuildOptions::default())
}

pub fn build_with(files: &[(&str, &str)], options: &BuildOptions) -> Built {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project");
    fs::create_dir_all(&root).unwrap();
    let mut has_stage = false;
    for (path, contents) in files {
        has_stage |= *path == "stage.gs";
        let p = root.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, contents).unwrap();
    }
    if !has_stage {
        fs::write(root.join("stage.gs"), "costumes \"blank.svg\";\n").unwrap();
    }
    fs::write(root.join("blank.svg"), BLANK_SVG).unwrap();
    let std_path = dir.path().join("std");
    fs::create_dir_all(&std_path).unwrap();
    build_in(dir, root, std_path, options)
}

pub fn build_in(
    dir: tempfile::TempDir,
    root: PathBuf,
    std_path: PathBuf,
    options: &BuildOptions,
) -> Built {
    // with an explicit --std-path, let build_impl resolve the standard library itself
    let stdlib = options.std_path.is_none().then(|| StandardLibrary {
        version: Version::new(0, 0, 0),
        path: std_path,
    });
    let mut sb3 = Vec::new();
    let artifact = build_impl(
        Rc::new(RefCell::new(RealFS)),
        root.canonicalize().unwrap(),
        Cursor::new(&mut sb3),
        stdlib,
        options,
    )
    .expect("build_impl failed");
    let project = if sb3.is_empty() {
        None
    } else {
        let mut zip = zip::ZipArchive::new(Cursor::new(&sb3)).unwrap();
        let mut s = String::new();
        zip.by_name("project.json")
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        Some(serde_json::from_str(&s).unwrap())
    };
    Built {
        artifact,
        sb3,
        project,
        dir,
    }
}

/// Build a single sprite called `main` from `src`.
pub fn build_main(src: &str) -> Built {
    build(&[("main.gs", &format!("costumes \"blank.svg\";\n{src}"))])
}

impl Built {
    pub fn project(&self) -> &Value {
        self.project.as_ref().unwrap_or_else(|| {
            panic!("build failed: {:#?}", self.diagnostics())
        })
    }

    pub fn target(&self, name: &str) -> &Value {
        self.project()["targets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("no target {name}"))
    }

    /// Initial value of the variable called `name` in target `target`.
    pub fn var(&self, target: &str, name: &str) -> Value {
        let vars = self.target(target)["variables"].as_object().unwrap();
        vars.values()
            .find(|v| v[0] == name)
            .unwrap_or_else(|| panic!("no variable {name} in {vars:?}"))[1]
            .clone()
    }

    pub fn list(&self, target: &str, name: &str) -> Value {
        let lists = self.target(target)["lists"].as_object().unwrap();
        lists
            .values()
            .find(|v| v[0] == name)
            .unwrap_or_else(|| panic!("no list {name}"))[1]
            .clone()
    }

    /// All blocks of target `target` with opcode `opcode`.
    pub fn blocks(&self, target: &str, opcode: &str) -> Vec<Value> {
        self.target(target)["blocks"]
            .as_object()
            .unwrap()
            .values()
            .filter(|b| b["opcode"] == opcode)
            .cloned()
            .collect()
    }

    /// Debug-formatted diagnostics of all targets.
    pub fn diagnostics(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .artifact
            .stage_diagnostics
            .diagnostics
            .iter()
            .map(|d| format!("{:?}", d.kind))
            .collect();
        for sd in self.artifact.sprites_diagnostics.values() {
            out.extend(sd.diagnostics.iter().map(|d| format!("{:?}", d.kind)));
        }
        out
    }

    pub fn failed(&self) -> bool {
        self.artifact.failure()
    }
}

/// The input value written for `input` of the first block with `opcode` in target `main`.
pub fn input(b: &Built, opcode: &str, input: &str) -> Value {
    let blocks = b.blocks("main", opcode);
    assert!(!blocks.is_empty(), "no {opcode} block");
    blocks[0]["inputs"][input].clone()
}
