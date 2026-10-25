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
