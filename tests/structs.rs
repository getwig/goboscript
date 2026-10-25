//! GBS-003: struct assignment from a struct literal is atomic.
mod common;

use common::*;
use serde_json::{
    json,
    Value,
};

/// The sequence of (variable name, VALUE input) of the `data_setvariableto` blocks under the
/// green flag script, in execution order.
fn set_sequence(b: &Built) -> Vec<(String, Value)> {
    let blocks = b.target("main")["blocks"].as_object().unwrap();
    let hat = blocks
        .iter()
        .find(|(_, v)| v["opcode"] == "event_whenflagclicked")
        .unwrap();
    let mut out = vec![];
    let mut next = hat.1["next"].as_str().map(str::to_owned);
    while let Some(id) = next {
        let block = &blocks[&id];
        if block["opcode"] == "data_setvariableto" {
            let value = &block["inputs"]["VALUE"];
            let value = if value[1].is_string() {
                blocks[value[1].as_str().unwrap()].clone()
            } else {
                value[1].clone()
            };
            out.push((block["fields"]["VARIABLE"][0].as_str().unwrap().to_owned(), value));
        }
        next = block["next"].as_str().map(str::to_owned);
    }
    out
}

#[test]
fn swap_through_struct_literal_uses_temporaries() {
    let b = build_main(
        "struct P { x, y }\nvar P a = P { x: 1, y: 2 };\nonflag { a = P { x: a.y, y: a.x }; }",
    );
    let seq = set_sequence(&b);
    let names: Vec<_> = seq.iter().map(|(n, _)| n.as_str()).collect();
    // both fields are read into temporaries before a.x / a.y are written
    assert_eq!(names.len(), 4, "{seq:?}");
    assert!(names[0].starts_with('@') && names[1].starts_with('@'), "{names:?}");
    assert_eq!(&names[2..], ["a.x", "a.y"]);
    assert_eq!(seq[0].1, json!([12, "a.y", "a.y"]));
    assert_eq!(seq[1].1, json!([12, "a.x", "a.x"]));
}

#[test]
fn struct_literal_without_self_reference_has_no_temporaries() {
    let b = build_main(
        "struct P { x, y }\nvar P a = P { x: 1, y: 2 };\nvar P c = P { x: 1, y: 2 };\n\
         onflag { a = P { x: c.y, y: 5 }; }",
    );
    let names: Vec<_> = set_sequence(&b).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["a.x", "a.y"]);
}

#[test]
fn struct_list_item_assignment_uses_temporaries() {
    let b = build_main(
        "struct P { x, y }\nlist P l;\n\
         onflag { add P { x: 1, y: 2 } to l; l[1] = P { x: l[1].y, y: l[1].x }; }",
    );
    let names: Vec<_> = set_sequence(&b).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names.iter().all(|n| n.starts_with('@')));
    let replaces = b.blocks("main", "data_replaceitemoflist");
    assert_eq!(replaces.len(), 2);
    for r in replaces {
        assert_eq!(r["inputs"]["ITEM"][1][0], json!(12), "item must read a temporary: {r}");
    }
}
