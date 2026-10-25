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

// All environment-dependent checks live in one test: tests in a binary run in parallel and
// share the process environment.
#[test]
fn ai_detection_and_its_opt_out() {
    std::env::set_var("CLAUDECODE", "1");
    // default: marker in the notes
    let b = build(&[("main.gs", MAIN)]);
    assert!(stage_comments(&b).iter().any(|n| n.contains("510P")));
    // opt-out: no marker, no AGENTS.md
    let opts = BuildOptions {
        no_ai_detection: true,
        ..Default::default()
    };
    let b = build_with(&[("main.gs", MAIN)], &opts);
    assert!(!b.dir.path().join("project/AGENTS.md").exists());
    assert!(!stage_comments(&b)[0].contains("510P"));
    // clean builds do not depend on the environment
    let clean = BuildOptions {
        no_ai_detection: true,
        no_stage_notes: true,
        no_turbowarp_config: true,
        ..Default::default()
    };
    let with_agent = build_with(&[("main.gs", MAIN)], &clean);
    std::env::remove_var("CLAUDECODE");
    let without_agent = build_with(&[("main.gs", MAIN)], &clean);
    assert_eq!(with_agent.project(), without_agent.project());
}

#[test]
fn stage_comments_can_be_disabled() {
    let b = build(&[("main.gs", MAIN)]);
    assert_eq!(stage_comments(&b).len(), 2);
    let only_tw = build_with(
        &[("main.gs", MAIN)],
        &BuildOptions {
            no_stage_notes: true,
            ..Default::default()
        },
    );
    let comments = stage_comments(&only_tw);
    assert_eq!(comments.len(), 1);
    assert!(comments[0].contains("_twconfig_"));
    let none = build_with(
        &[("main.gs", MAIN)],
        &BuildOptions {
            no_stage_notes: true,
            no_turbowarp_config: true,
            ..Default::default()
        },
    );
    assert!(stage_comments(&none).is_empty());
    // scripts start at the left edge when there are no comments
    let hat = &none.blocks("main", "event_whenflagclicked")[0];
    assert_eq!(hat["x"], 0);
}
