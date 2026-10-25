//! GBS-005: macro arguments that are evaluated more than once are reported.
mod common;

use common::*;

#[test]
fn repeated_macro_argument_with_side_effects_warns() {
    let b = build_main(
        "%define MIN(A, B) ((A) - ((A) - (B)) * ((A) > (B)))\n\
         var calls = 0;\nvar m = 0;\nvar a = 1;\n\
         func bump() { calls += 1; return calls; }\n\
         onflag { m = MIN(bump(), 100); m = MIN(a, 100); }",
    );
    assert!(!b.failed(), "{:?}", b.diagnostics());
    let diags: Vec<_> = b
        .diagnostics()
        .into_iter()
        .filter(|d| d.starts_with("MacroArgumentEvaluatedMultipleTimes"))
        .collect();
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert!(diags[0].contains("param: \"A\"") && diags[0].contains("uses: 3"));
}

#[test]
fn macro_warning_does_not_stop_the_build() {
    let b = build_main(
        "%define TWICE(A) ((A) + (A))\nvar n = 0;\nonflag { n = TWICE(n + 1); }",
    );
    assert!(!b.failed());
    assert!(!b.sb3.is_empty(), "a warning must not suppress the output");
}
