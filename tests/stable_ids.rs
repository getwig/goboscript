//! GBS-022: with stable ids, editing one script keeps the block ids of the others.
mod common;

use std::collections::BTreeMap;

use common::*;
use libgoboscript::codegen::build::BuildOptions;

fn opts() -> BuildOptions {
    BuildOptions {
        stable_ids: true,
        ..Default::default()
    }
}

/// block id -> opcode of all blocks under the script whose hat has `hat_opcode` (and, for
/// broadcasts, message), found by walking `next`/`parent` links.
fn ids_by_script(b: &Built, target: &str) -> BTreeMap<String, String> {
    b.target(target)["blocks"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(id, block)| (id.clone(), block["opcode"].as_str().unwrap().to_owned()))
        .collect()
}

const OTHER: &str = "costumes \"blank.svg\";\nonflag { say \"other\"; }";

fn main_src(extra: &str) -> String {
    format!(
        "costumes \"blank.svg\";\n\
         proc p n {{ say $n; }}\n\
         onflag {{ say 1;{extra} p 2; }}\n\
         on \"go\" {{ say 3; move 10; }}\n"
    )
}

#[test]
fn stable_ids_survive_edits_in_other_scripts() {
    let a = build_with(&[("main.gs", &main_src("")), ("other.gs", OTHER)], &opts());
    let b = build_with(
        &[("main.gs", &main_src(" say 99; say 98;")), ("other.gs", OTHER)],
        &opts(),
    );
    let ids_a = ids_by_script(&a, "main");
    let ids_b = ids_by_script(&b, "main");
    // the broadcast script and the procedure keep all their ids
    let kept: Vec<_> = ids_a
        .iter()
        .filter(|(_, op)| {
            matches!(
                op.as_str(),
                "event_whenbroadcastreceived" | "motion_movesteps" | "procedures_definition"
            )
        })
        .collect();
    assert_eq!(kept.len(), 3);
    for (id, op) in kept {
        assert_eq!(ids_b.get(id), Some(op), "{id} {op} changed");
    }
    // other targets are unaffected
    assert_eq!(ids_by_script(&a, "other"), ids_by_script(&b, "other"));
    // deterministic
    let a2 = build_with(&[("main.gs", &main_src("")), ("other.gs", OTHER)], &opts());
    assert_eq!(a.project(), a2.project());
}

#[test]
fn default_ids_are_sequential() {
    let a = build_with(&[("main.gs", &main_src(""))], &BuildOptions::default());
    let ids = ids_by_script(&a, "main");
    assert!(ids.keys().all(|id| id.len() <= 3), "{ids:?}");
    let s = build_with(&[("main.gs", &main_src(""))], &opts());
    assert!(ids_by_script(&s, "main").keys().any(|id| id.len() > 3));
}
