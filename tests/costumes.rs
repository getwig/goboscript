//! GBS-006: costume/backdrop references that Scratch accepts are not build errors.
mod common;

use common::*;

#[test]
fn costume_numbers_and_special_names_are_accepted() {
    let b = build(&[
        (
            "main.gs",
            "costumes \"blank.svg\" as \"a\", \"blank.svg\" as \"b\";\n\
             onflag { switch_costume 2; switch_costume \"2\"; switch_costume \"next costume\"; \
             switch_costume \"previous costume\"; switch_backdrop \"next backdrop\"; \
             switch_backdrop 1; switch_backdrop \"random backdrop\"; }",
        ),
    ]);
    assert!(!b.failed(), "{:?}", b.diagnostics());
    assert!(b.diagnostics().is_empty(), "{:?}", b.diagnostics());
}

#[test]
fn unknown_costume_name_is_a_warning() {
    let b = build(&[(
        "main.gs",
        "costumes \"blank.svg\" as \"a\";\nonflag { switch_costume \"Head\"; switch_backdrop \"nope\"; }",
    )]);
    assert!(!b.failed(), "{:?}", b.diagnostics());
    let diags = b.diagnostics();
    assert!(diags.iter().any(|d| d.contains("InvalidCostumeName")), "{diags:?}");
    assert!(diags.iter().any(|d| d.contains("InvalidBackdropName")), "{diags:?}");
}
