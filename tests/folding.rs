//! GBS-002: algebraic simplifications must not drop Scratch's number/text casts.
mod common;

use common::*;
use serde_json::json;

fn count(b: &Built, opcode: &str) -> usize {
    b.blocks("main", opcode).len()
}

#[test]
fn casts_on_variables_are_kept() {
    let b = build_main(
        "var s = \"007\";\nvar r = 0;\n\
         onflag { r = s + 0; r = 0 + s; r = s - 0; r = s * 1; r = 1 * s; r = s / 1; r = s * 0; r = 0 * s; r = \"\" & s; r = s & \"\"; }",
    );
    assert_eq!(count(&b, "operator_add"), 2);
    assert_eq!(count(&b, "operator_subtract"), 1);
    assert_eq!(count(&b, "operator_multiply"), 4);
    assert_eq!(count(&b, "operator_divide"), 1);
    assert_eq!(count(&b, "operator_join"), 2);
}

#[test]
fn casts_on_arithmetic_results_are_kept() {
    // (a/b) may be NaN or -0 or Infinity, where + 0 and * 0 are not identities in Scratch
    let b = build_main("var a = 0;\nvar r = 0;\nonflag { r = a / a + 0; r = (a / a) * 0; }");
    assert_eq!(count(&b, "operator_add"), 1);
    assert_eq!(count(&b, "operator_multiply"), 1);
}

#[test]
fn safe_simplifications_still_apply() {
    let b = build_main(
        "var s = \"ab\";\nvar r = 0;\n\
         onflag { r = round(s) * 1; r = length(s) + 0; r = length(s) * 0; r = (s & s) & \"\"; }",
    );
    assert_eq!(count(&b, "operator_multiply"), 0);
    assert_eq!(count(&b, "operator_add"), 0);
    assert_eq!(count(&b, "operator_join"), 1);
    // literal-only expressions are folded with Scratch semantics
    let b = build_main("var r = 0;\nonflag { r = \"007\" + 0; }");
    assert_eq!(input(&b, "data_setvariableto", "VALUE"), json!([1, [4, 7]]));
}

// Constant comparisons rewritten to `not (...)` (<=, >=, !=) still fold.
#[test]
fn rewritten_comparisons_fold() {
    let b = build_main("onflag { say 1 <= 2; say 3 >= 4; say 1 != 1; }");
    assert_eq!(count(&b, "operator_not"), 0);
    assert_eq!(count(&b, "operator_gt") + count(&b, "operator_lt") + count(&b, "operator_equals"), 0);
}
