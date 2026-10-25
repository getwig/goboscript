//! GBS-029: locals that are shared by overlapping calls are reported.
mod common;

use common::*;
use libgoboscript::codegen::build::BuildOptions;

const SRC: &str = "costumes \"blank.svg\";\n\
proc safe n { local t = $n; say t; }\n\
nowarp proc yielding n { local t = $n; wait 1; say t; }\n\
proc recurse n { local t = $n; if $n > 0 { recurse $n - 1; } say t; }\n\
proc ping n { local t = $n; if $n > 0 { pong $n - 1; } }\n\
proc pong n { if $n > 0 { ping $n - 1; } }\n\
func fact(n) { local t = $n; if $n < 2 { return 1; } return t * fact($n - 1); }\n\
onflag { safe 1; yielding 1; recurse 3; ping 3; say fact(3); }";

fn unsafe_locals(b: &Built) -> Vec<String> {
    b.diagnostics()
        .into_iter()
        .filter(|d| d.starts_with("UnsafeLocal"))
        .collect()
}

#[test]
fn locals_in_nowarp_or_recursive_procedures_are_warnings() {
    let b = build(&[("main.gs", SRC)]);
    assert!(!b.failed(), "{:?}", b.diagnostics());
    let diags = unsafe_locals(&b);
    assert_eq!(diags.len(), 4, "{diags:#?}");
    for (procedure, reason) in [
        ("yielding", "NoWarp"),
        ("recurse", "Recursive"),
        ("ping", "Recursive"),
        ("fact", "Recursive"),
    ] {
        assert!(
            diags
                .iter()
                .any(|d| d.contains(&format!("procedure: \"{procedure}\"")) && d.contains(reason)),
            "{procedure}: {diags:#?}"
        );
    }
    assert!(!diags.iter().any(|d| d.contains("\"safe\"")));
}

#[test]
fn deny_unsafe_locals_makes_them_errors() {
    let b = build_with(
        &[("main.gs", SRC)],
        &BuildOptions {
            deny_unsafe_locals: true,
            ..Default::default()
        },
    );
    assert!(b.failed());
    assert!(b.sb3.is_empty());
}
