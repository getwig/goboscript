//! GBS-029: `local` variables compile to one sprite variable per procedure, not one per call.
//! They are only correct when a call cannot overlap with another call of the same procedure:
//! report locals in procedures that can yield (`nowarp`) or that are on a call cycle
//! (recursion).

use rustc_hash::{
    FxHashMap,
    FxHashSet,
};

use crate::{
    ast::*,
    diagnostic::{
        DiagnosticKind,
        SpriteDiagnostics,
        UnsafeLocalReason,
    },
    misc::SmolStr,
};

pub fn visit_project(
    project: &Project,
    stage_diagnostics: &mut SpriteDiagnostics,
    sprites_diagnostics: &mut FxHashMap<SmolStr, SpriteDiagnostics>,
    deny: bool,
) {
    visit_sprite(&project.stage, stage_diagnostics, deny);
    for (sprite_name, sprite) in &project.sprites {
        visit_sprite(sprite, sprites_diagnostics.get_mut(sprite_name).unwrap(), deny);
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Callable {
    Proc(SmolStr),
    Func(SmolStr),
}

fn callees(sprite: &Sprite, callable: &Callable) -> Vec<Callable> {
    let references = match callable {
        Callable::Proc(name) => sprite.proc_references.get(name),
        Callable::Func(name) => sprite.func_references.get(name),
    };
    let Some(references) = references else {
        return vec![];
    };
    references
        .procs
        .iter()
        .filter(|name| sprite.procs.contains_key(*name))
        .map(|name| Callable::Proc(name.clone()))
        .chain(
            references
                .funcs
                .iter()
                .filter(|name| sprite.funcs.contains_key(*name))
                .map(|name| Callable::Func(name.clone())),
        )
        .collect()
}

/// Whether `start` can (directly or indirectly) call itself.
fn is_recursive(sprite: &Sprite, start: &Callable) -> bool {
    let mut seen: FxHashSet<Callable> = Default::default();
    let mut stack = callees(sprite, start);
    while let Some(callable) = stack.pop() {
        if &callable == start {
            return true;
        }
        if seen.insert(callable.clone()) {
            stack.extend(callees(sprite, &callable));
        }
    }
    false
}

fn sorted_locals(locals: Option<&FxHashMap<SmolStr, Var>>) -> Vec<&Var> {
    let mut locals: Vec<&Var> = locals.map(|l| l.values().collect()).unwrap_or_default();
    locals.sort_by_key(|var| var.span.start);
    locals
}

fn visit_sprite(sprite: &Sprite, d: &mut SpriteDiagnostics, deny: bool) {
    let mut procs: Vec<&Proc> = sprite.procs.values().collect();
    procs.sort_by_key(|proc| proc.span.start);
    for proc in procs {
        let locals = sorted_locals(sprite.proc_locals.get(&proc.name));
        if locals.is_empty() {
            continue;
        }
        let reason = if is_recursive(sprite, &Callable::Proc(proc.name.clone())) {
            Some(UnsafeLocalReason::Recursive)
        } else if !proc.warp {
            Some(UnsafeLocalReason::NoWarp)
        } else {
            None
        };
        if let Some(reason) = reason {
            for var in locals {
                d.report(
                    DiagnosticKind::UnsafeLocal {
                        name: var.name.clone(),
                        procedure: proc.name.clone(),
                        reason,
                        deny,
                    },
                    &var.span,
                );
            }
        }
    }
    let mut funcs: Vec<&Func> = sprite.funcs.values().collect();
    funcs.sort_by_key(|func| func.span.start);
    for func in funcs {
        if !is_recursive(sprite, &Callable::Func(func.name.clone())) {
            continue;
        }
        for var in sorted_locals(sprite.func_locals.get(&func.name)) {
            d.report(
                DiagnosticKind::UnsafeLocal {
                    name: var.name.clone(),
                    procedure: func.name.clone(),
                    reason: UnsafeLocalReason::Recursive,
                    deny,
                },
                &var.span,
            );
        }
    }
}
