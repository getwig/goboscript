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

// GBS-038: an item operation on a struct list is one operation per field; the index must be
// evaluated once, otherwise `m[random(1, 6)] = P {..}` writes the fields to different items.
#[test]
fn struct_list_index_is_evaluated_once() {
    let b = build_main(
        "struct P { x, y }\nlist P m;\nvar i = 1;\n\
         onflag { m[random(1, 6)] = P { x: 7, y: 8 }; insert P { x: 1, y: 2 } at m[i + 1]; \
         delete m[\"random\"]; insert P { x: 3, y: 4 } at m[\"any\"]; m[i] = P { x: 5, y: 6 }; delete m[2]; }",
    );
    // one pick random per random index (the literal "random"/"any" become pick random too)
    assert_eq!(b.blocks("main", "operator_random").len(), 3);
    // every non-trivial index went into a temporary
    let temps = b
        .blocks("main", "data_setvariableto")
        .iter()
        .filter(|s| s["fields"]["VARIABLE"][0].as_str().unwrap().starts_with('@'))
        .count();
    assert_eq!(temps, 4);
    // per-field operations read the temporary (or the plain variable / literal), never an expression
    for opcode in ["data_replaceitemoflist", "data_insertatlist", "data_deleteoflist"] {
        for block in b.blocks("main", opcode) {
            let index = &block["inputs"]["INDEX"];
            assert!(
                index[1][0] == 12 || index[0] == 1,
                "{opcode} index must be a variable or a literal: {index}"
            );
        }
    }
    // the literal "random" for delete is random(1, length of the first field list)
    assert_eq!(b.blocks("main", "data_lengthoflist").len(), 2);
}
