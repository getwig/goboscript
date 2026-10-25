//! GBS-025 / GBS-026: builds can be made independent of the environment and free of the
//! generated Stage comments.
mod common;

use common::*;
use libgoboscript::codegen::build::BuildOptions;

const MAIN: &str = "costumes \"blank.svg\";\nonflag { say 1; }";

fn stage_comments(b: &Built) -> Vec<String> {
    b.target("Stage")["comments"]
        .as_object()
        .map(|c| c.values().map(|v| v["text"].as_str().unwrap().to_owned()).collect())
        .unwrap_or_default()
}

#[test]
fn no_ai_detection_writes_no_marker_and_no_agents_md() {
    // simulate an agent environment for this test process
    std::env::set_var("CLAUDECODE", "1");
    let b = build_with(
        &[("main.gs", MAIN)],
        &BuildOptions {
            no_ai_detection: true,
            ..Default::default()
        },
    );
    assert!(!b.dir.path().join("project/AGENTS.md").exists());
    let notes = &stage_comments(&b)[0];
    assert!(!notes.contains("510P"), "{notes}");
}

#[test]
fn ai_detection_is_on_by_default() {
    std::env::set_var("CLAUDECODE", "1");
    let b = build(&[("main.gs", MAIN)]);
    assert!(stage_comments(&b).iter().any(|n| n.contains("510P")));
}
