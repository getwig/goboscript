//! Number literals and constant folding keep Scratch/JavaScript number semantics.
mod common;

use common::*;
use serde_json::json;

// GBS-001: integral numbers >= 2^63 must not saturate to i64::MAX.
#[test]
fn big_integral_numbers_are_not_saturated() {
    let b = build_main(
        "var big = 1e+21;\nvar said = 0;\nlist items = [1e+21, 1e+300, 9007199254740992];\n\
         onflag { said = 1e+21; say 1e+21 + 1; }",
    );
    assert_eq!(b.var("main", "big"), json!(1e21));
    assert_eq!(b.list("main", "items"), json!([1e21, 1e300, 9007199254740992i64]));
    let set = b.blocks("main", "data_setvariableto");
    assert_eq!(set[0]["inputs"]["VALUE"], json!([1, [4, 1e21]]));
    assert_eq!(input(&b, "looks_say", "MESSAGE"), json!([1, [4, 1e21]]));
}

// GBS-001: joins fold numbers with JavaScript's Number#toString.
#[test]
fn join_folding_uses_javascript_number_to_string() {
    let b = build_main("onflag { say \"x\" & 1e+21; say \"y\" & 0.1; say \"z\" & 100; }");
    let says: Vec<_> = b
        .blocks("main", "looks_say")
        .iter()
        .map(|b| b["inputs"]["MESSAGE"][1][1].clone())
        .collect();
    for expected in ["x1e+21", "y0.1", "z100"] {
        assert!(says.contains(&json!(expected)), "{expected} not in {says:?}");
    }
}

// GBS-007: exponent without sign and integer literals beyond i64 are valid number literals.
#[test]
fn exponent_without_sign_and_huge_integers_lex() {
    let b = build_main("var a = 1e21;\nvar b = 2E3;\nvar c = 100000000000000000000000;\nvar d = 1_000;");
    assert!(!b.failed(), "{:?}", b.diagnostics());
    assert_eq!(b.var("main", "a"), json!(1e21));
    assert_eq!(b.var("main", "b"), json!(2000));
    assert_eq!(b.var("main", "c"), json!(1e23));
    assert_eq!(b.var("main", "d"), json!(1000));
}

// GBS-015: float literals survive the project.json round trip bit-exactly.
#[test]
fn float_literals_keep_full_precision() {
    let b = build_main(
        "var a = 111.00000000000001;\nvar c = 0.30000000000000004;\nonflag { set_size 111.00000000000001; }",
    );
    assert_eq!(b.var("main", "a").as_f64().unwrap().to_bits(), 111.00000000000001f64.to_bits());
    assert_eq!(b.var("main", "c").as_f64().unwrap().to_bits(), 0.30000000000000004f64.to_bits());
    let size = input(&b, "looks_setsizeto", "SIZE")[1][1].as_f64().unwrap();
    assert_eq!(size.to_bits(), 111.00000000000001f64.to_bits());
}

// GBS-021: constant NaN is not folded into the text "NaN"; -0.0 keeps its sign.
#[test]
fn nan_is_computed_at_runtime_and_negative_zero_is_kept() {
    let b = build_main("list items = [-0.0];\nvar z = -0.0;\nonflag { add 0/0 to items; say -0.0; say sqrt(-1); }");
    assert!(b.list("main", "items")[0].as_f64().unwrap().is_sign_negative());
    assert!(b.var("main", "z").as_f64().unwrap().is_sign_negative());
    let div = b.blocks("main", "operator_divide");
    assert_eq!(div.len(), 1, "0/0 must stay a division block");
    assert_eq!(b.blocks("main", "operator_mathop").len(), 1, "sqrt(-1) must not be folded");
    let says = b.blocks("main", "looks_say");
    assert!(says.iter().any(|s| s["inputs"]["MESSAGE"][1][1]
        .as_f64()
        .is_some_and(|n| n == 0.0 && n.is_sign_negative())));
    // ordinary constant folding still happens
    let b = build_main("onflag { say 1/4 + 1; }");
    assert_eq!(input(&b, "looks_say", "MESSAGE"), json!([1, [4, 1.25]]));
}
