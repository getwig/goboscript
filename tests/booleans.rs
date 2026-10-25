//! GBS-014: boolean literals behave like Scratch booleans.
mod common;

use common::*;
use serde_json::json;

#[test]
fn boolean_literals_show_and_join_as_text() {
    let b = build_main(
        "var flag = true;\nvar off = false;\nvar x = 0;\nonflag { say true; say \"x\" & true; x = false; }",
    );
    assert_eq!(b.var("main", "flag"), json!("true"));
    assert_eq!(b.var("main", "off"), json!("false"));
    let says: Vec<_> = b
        .blocks("main", "looks_say")
        .iter()
        .map(|s| s["inputs"]["MESSAGE"].clone())
        .collect();
    assert!(says.contains(&json!([1, [10, "true"]])), "{says:?}");
    assert!(says.contains(&json!([1, [10, "xtrue"]])), "{says:?}");
    assert_eq!(
        input(&b, "data_setvariableto", "VALUE"),
        json!([1, [10, "false"]])
    );
}

#[test]
fn boolean_literals_in_arithmetic_are_one_and_zero() {
    let b = build_main("var x = 0;\nonflag { say x + true; say true + 1; say -true; }");
    let add = b.blocks("main", "operator_add");
    assert_eq!(add.len(), 1);
    assert_eq!(add[0]["inputs"]["NUM2"], json!([1, [4, 1]]));
    let says: Vec<_> = b
        .blocks("main", "looks_say")
        .iter()
        .map(|s| s["inputs"]["MESSAGE"].clone())
        .collect();
    assert!(says.contains(&json!([1, [4, 2]])), "{says:?}");
    assert!(says.contains(&json!([1, [4, -1]])), "{says:?}");
}

#[test]
fn non_boolean_condition_still_compares_with_one() {
    let b = build_main("var x = 1;\nonflag { if x { say 1; } }");
    let eq = b.blocks("main", "operator_equals");
    assert_eq!(eq.len(), 1);
    assert_eq!(eq[0]["inputs"]["OPERAND2"], json!([1, [4, 1]]));
}

#[test]
fn folded_boolean_conditions_keep_their_truth_value() {
    let b = build_main("onflag { until 1 < 2 {} if true { say 1; } if false { say 2; } }");
    for eq in b.blocks("main", "operator_equals") {
        let lhs = &eq["inputs"]["OPERAND1"][1];
        assert_eq!(lhs[0], json!(4), "condition operand must be a number: {eq}");
    }
}
