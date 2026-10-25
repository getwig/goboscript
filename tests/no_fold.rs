//! `--no-fold`: no constant folding, no algebraic simplification, no dead-code removal, so the
//! output means exactly what the source says (GBS-031, GBS-032, GBS-034 come from folding;
//! GBS-023 from dead-code removal). Default builds keep upstream behaviour.
mod common;

use common::*;
use libgoboscript::codegen::build::BuildOptions;
use serde_json::json;

fn no_fold() -> BuildOptions {
    BuildOptions {
        no_fold: true,
        ..Default::default()
    }
}

fn build_nf(src: &str) -> Built {
    build_with(
        &[("main.gs", &format!("costumes \"blank.svg\";\n{src}"))],
        &no_fold(),
    )
}

fn count(b: &Built, opcode: &str) -> usize {
    b.blocks("main", opcode).len()
}

#[test]
fn constant_arithmetic_is_not_folded() {
    // GBS-031: text operands are cast by Scratch at runtime, not by the compiler
    let b = build_nf("var r = 0;\nonflag { r = \"12abc\" + 1; r = \"0x10\" * 2; r = 7 / 0; r = 0 / 0; }");
    assert_eq!(count(&b, "operator_add"), 1);
    assert_eq!(count(&b, "operator_multiply"), 1);
    assert_eq!(count(&b, "operator_divide"), 2);
    let add = &b.blocks("main", "operator_add")[0];
    assert_eq!(add["inputs"]["NUM1"], json!([1, [10, "12abc"]]));
    assert_eq!(add["inputs"]["NUM2"], json!([1, [4, 1]]));
    // default: folded
    let d = build_main("var r = 0;\nonflag { r = \"12abc\" + 1; }");
    assert_eq!(d.blocks("main", "operator_add").len(), 0);
}

#[test]
fn constant_contains_is_not_folded() {
    // GBS-032: `"abc" in "xabcx"` is `"xabcx" contains "abc"` (true); the folder computed it
    // with the operands swapped
    let b = build_nf("onflag { say \"abc\" in \"xabcx\"; }");
    let c = b.blocks("main", "operator_contains");
    assert_eq!(c.len(), 1);
    assert_eq!(c[0]["inputs"]["STRING1"], json!([1, [10, "xabcx"]]));
    assert_eq!(c[0]["inputs"]["STRING2"], json!([1, [10, "abc"]]));
}

#[test]
fn constant_join_keeps_operands() {
    // GBS-034: `"x" & 2.50` was folded to "x2.5"
    let b = build_nf("onflag { say \"x\" & 2.50; say 1 & 2; }");
    assert_eq!(count(&b, "operator_join"), 2);
}

#[test]
fn simplifications_are_off() {
    let b = build_nf(
        "var s = \"ab\";\nvar r = 0;\n\
         onflag { r = round(s) * 1; r = length(s) + 0; r = length(s) * 0; r = (s & s) & \"\"; r = 1 + 2; }",
    );
    assert_eq!(count(&b, "operator_multiply"), 2);
    assert_eq!(count(&b, "operator_add"), 2);
    assert_eq!(count(&b, "operator_join"), 2);
}

#[test]
fn comparisons_are_lowered_but_not_folded() {
    let b = build_nf("onflag { say 1 <= 2; say 3 >= 4; say 1 != 1; say 1 < 2; }");
    assert_eq!(count(&b, "operator_not"), 3);
    assert_eq!(count(&b, "operator_gt"), 1);
    assert_eq!(count(&b, "operator_lt"), 2);
    assert_eq!(count(&b, "operator_equals"), 1);
    assert_eq!(count(&b, "looks_say"), 4);
}

#[test]
fn negative_literals_stay_literals() {
    let b = build_nf("var v = 1;\nvar r = 0;\nonflag { r = -5; r = -0.0; r = -v; r = 3 // 2; }");
    let sets = b.blocks("main", "data_setvariableto");
    let values: Vec<_> = sets.iter().map(|s| s["inputs"]["VALUE"].clone()).collect();
    assert!(values.contains(&json!([1, [10, "-5"]])) || values.contains(&json!([1, [4, -5]])), "{values:?}");
    // -v is 0 - v, `//` is floor(a / b)
    assert_eq!(count(&b, "operator_subtract"), 1);
    assert_eq!(count(&b, "operator_mathop"), 1);
    assert_eq!(count(&b, "operator_divide"), 1);
}

#[test]
fn unused_procedures_are_kept() {
    // GBS-023: dead-code removal is off; locals of the kept procedure are declared
    let src = "proc helper n { local t = $n * 2; say t; }\nfunc twice(n) { return $n * 2; }\nonflag { say 1; }";
    let b = build_nf(src);
    assert_eq!(count(&b, "procedures_definition"), 2);
    assert!(!b
        .diagnostics()
        .iter()
        .any(|d| d.starts_with("UnusedProc") || d.starts_with("UnusedFunc")));
    let d = build_main(src);
    assert_eq!(d.blocks("main", "procedures_definition").len(), 0);
}

#[test]
fn default_still_folds() {
    let d = build_main("var r = 0;\nonflag { r = 1 + 2; say 1 <= 2; }");
    assert_eq!(d.blocks("main", "operator_add").len(), 0);
    assert_eq!(d.blocks("main", "operator_not").len(), 0);
}
