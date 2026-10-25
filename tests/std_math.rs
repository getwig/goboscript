//! GBS-004: the vendored std/math macros give correct results. Literal arguments are constant
//! folded with Scratch semantics, so the folded value is what Scratch would compute.
mod common;

use common::*;
use libgoboscript::codegen::build::BuildOptions;

fn std_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("std")
}

/// Evaluate each expression (with `%include std/math`) and return the folded numbers.
fn eval(exprs: &[&str]) -> Vec<f64> {
    let body: String = exprs.iter().map(|e| format!("say {e};\n")).collect();
    let b = build_with(
        &[(
            "main.gs",
            &format!("%include std/math\ncostumes \"blank.svg\";\nonflag {{\n{body}}}"),
        )],
        &BuildOptions {
            std_path: Some(std_dir()),
            ..Default::default()
        },
    );
    assert!(!b.failed(), "{:?}", b.diagnostics());
    let blocks = b.target("main")["blocks"].as_object().unwrap();
    let hat = blocks
        .values()
        .find(|v| v["opcode"] == "event_whenflagclicked")
        .unwrap();
    let mut out = vec![];
    let mut next = hat["next"].as_str();
    while let Some(id) = next {
        let message = &blocks[id]["inputs"]["MESSAGE"];
        let value = &message[1][1];
        out.push(
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
                .unwrap_or_else(|| panic!("not folded to a number: {message}")),
        );
        next = blocks[id]["next"].as_str();
    }
    out
}

fn check(cases: &[(&str, f64)]) {
    let exprs: Vec<&str> = cases.iter().map(|(e, _)| *e).collect();
    for ((expr, expected), actual) in cases.iter().zip(eval(&exprs)) {
        let tol = 1e-12 * expected.abs().max(1.0);
        assert!(
            (actual - expected).abs() <= tol,
            "{expr}: expected {expected}, got {actual}"
        );
    }
}

#[test]
fn min_max_are_exact() {
    check(&[
        ("MIN(1, 2)", 1.0),
        ("MIN(2, 1)", 1.0),
        ("MIN(1e17, 1)", 1.0),
        ("MIN(1, 1e17)", 1.0),
        ("MIN(-5, -5)", -5.0),
        ("MAX(1e17, 1)", 1e17),
        ("MAX(1, 1e17)", 1e17),
        ("MAX(-3, 2)", 2.0),
        ("MAX(0.1, 0.30000000000000004)", 0.30000000000000004),
    ]);
}

#[test]
fn clamp_returns_the_bounds() {
    check(&[
        ("CLAMP(5, 10, 20)", 10.0),
        ("CLAMP(15, 10, 20)", 15.0),
        ("CLAMP(25, 10, 20)", 20.0),
        ("CLAMP(10, 10, 20)", 10.0),
        ("CLAMP(20, 10, 20)", 20.0),
        ("CLAMP(-1e17, -1, 1)", -1.0),
    ]);
}

#[test]
fn map_and_invlerp_follow_their_documentation() {
    check(&[
        ("MAP(0, 10, 0, 100, 0)", 0.0),
        ("MAP(0, 10, 0, 100, 10)", 100.0),
        ("MAP(0, 10, 0, 100, 2.5)", 25.0),
        ("MAP(-1, 1, 0, 1, 0)", 0.5),
        ("2 / INVLERP(4, 0, 8)", 4.0),
        ("INVLERP(6, 4, 8) * 2", 1.0),
        ("LERP(0, 10, 0.25)", 2.5),
    ]);
}

#[test]
fn hyperbolic_and_power_functions() {
    check(&[
        ("1 / ATANH(0.5)", 1.0 / 0.5f64.atanh()),
        ("2 * ACOSH(2)", 2.0 * 2f64.acosh()),
        ("2 * ASINH(1)", 2.0 * 1f64.asinh()),
        ("POW(2, 10)", 1024.0),
        ("POW(-2, 2)", 4.0),
        ("POW(-2, 3)", -8.0),
        ("POW(-2, -1)", -0.5),
        ("POW(9, 0.5)", 3.0),
        ("POW(0, 2)", 0.0),
        ("1 / MAG(3, 4)", 0.2),
        ("1 / DIST(0, 0, 3, 4)", 0.2),
        ("HEX(\"ff\")", 255.0),
        ("BIN(\"101\")", 5.0),
    ]);
}
