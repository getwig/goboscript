//! GBS-014: boolean literals behave like Scratch booleans.
mod common;

use common::*;
use serde_json::json;

/// `<(1) = (1)>` or `<(0) = (1)>`: the block a boolean literal input compiles to (GBS-035).
fn is_boolean_block(b: &Built, input: &serde_json::Value, value: bool) -> bool {
    let Some(id) = input[1].as_str() else {
        return false;
    };
    let block = &b.target("main")["blocks"][id];
    block["opcode"] == "operator_equals"
        && block["inputs"]["OPERAND1"] == json!([1, [10, if value { "1" } else { "0" }]])
        && block["inputs"]["OPERAND2"] == json!([1, [10, "1"]])
}

#[test]
fn boolean_literals_are_scratch_booleans() {
    // GBS-035 (regression of the first GBS-014 fix): initial values are JSON booleans, inputs
    // get a boolean reporter; the text "true" would cast to 0 and not equal 1.
    let b = build_main(
        "var flag = true;\nvar off = false;\nvar x = 0;\nlist l = [true, 1];\n\
         proc f n { say $n; }\n\
         onflag { say true; x = false; f true; add true to l; }",
    );
    assert_eq!(b.var("main", "flag"), json!(true));
    assert_eq!(b.var("main", "off"), json!(false));
    assert_eq!(b.list("main", "l"), json!([true, 1]));
    assert!(b
        .blocks("main", "looks_say")
        .iter()
        .any(|say| is_boolean_block(&b, &say["inputs"]["MESSAGE"], true)));
    let set = input(&b, "data_setvariableto", "VALUE");
    assert!(is_boolean_block(&b, &set, false), "{set}");
    let add = input(&b, "data_addtolist", "ITEM");
    assert!(is_boolean_block(&b, &add, true), "{add}");
    let call = &b.blocks("main", "procedures_call")[0];
    let arg = call["inputs"].as_object().unwrap().values().next().unwrap().clone();
    assert!(is_boolean_block(&b, &arg, true), "{arg}");
}

#[test]
fn constant_join_with_a_boolean_is_text() {
    let b = build_main("onflag { say \"x\" & true; }");
    assert_eq!(input(&b, "looks_say", "MESSAGE"), json!([1, [10, "xtrue"]]));
}

#[test]
fn folded_comparison_in_a_value_is_a_boolean() {
    // `x = 1 < 2` folds to the boolean true; it must stay a boolean (not the text "true")
    let b = build_main("var x = 0;\nonflag { x = 1 < 2; }");
    let set = input(&b, "data_setvariableto", "VALUE");
    assert!(is_boolean_block(&b, &set, true), "{set}");
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
