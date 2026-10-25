use rustc_hash::FxHashMap;

use crate::{
    ast::*,
    blocks::{
        BinOp,
        Repr,
        UnOp,
    },
    misc::SmolStr,
};

struct S<'a> {
    vars: &'a mut FxHashMap<SmolStr, Var>,
    /// Field names of the struct lists visible here (sprite and Stage), for GBS-038.
    struct_lists: &'a FxHashMap<SmolStr, Vec<SmolStr>>,
    callsites: &'a mut usize,
    funcs: &'a FxHashMap<SmolStr, Func>,
    func: Option<&'a Func>,
}

pub fn visit_project(project: &mut Project) {
    let mut callsites = 0;
    let stage_struct_lists = struct_lists(&project.stage, None);
    visit_sprite(&mut project.stage, &mut callsites, &stage_struct_lists);
    for sprite in project.sprites.values_mut() {
        let lists = struct_lists(sprite, Some(&project.stage));
        visit_sprite(sprite, &mut callsites, &lists);
    }
}

/// Struct-typed lists (name -> field names) of `sprite`, and of the Stage for a sprite.
fn struct_lists(sprite: &Sprite, stage: Option<&Sprite>) -> FxHashMap<SmolStr, Vec<SmolStr>> {
    let mut out = FxHashMap::default();
    for owner in stage.into_iter().chain(std::iter::once(sprite)) {
        for list in owner.lists.values() {
            let Some((type_name, _)) = list.type_.struct_() else {
                continue;
            };
            let struct_ = sprite
                .structs
                .get(type_name)
                .or_else(|| stage.and_then(|stage| stage.structs.get(type_name)));
            if let Some(struct_) = struct_ {
                out.insert(
                    list.name.clone(),
                    struct_.fields.iter().map(|f| f.name.clone()).collect(),
                );
            }
        }
    }
    out
}

fn visit_sprite(
    sprite: &mut Sprite,
    callsites: &mut usize,
    struct_lists: &FxHashMap<SmolStr, Vec<SmolStr>>,
) {
    let old_callsites = *callsites;
    for proc in sprite.procs.values_mut() {
        let proc_definition = sprite.proc_definitions.get_mut(&proc.name).unwrap();
        visit_stmts(
            proc_definition,
            &mut S {
                vars: &mut sprite.vars,
                struct_lists,
                callsites,
                funcs: &sprite.funcs,
                func: None,
            },
        );
    }
    for func in sprite.funcs.values() {
        let func_definition = sprite.func_definitions.get_mut(&func.name).unwrap();
        visit_stmts(
            func_definition,
            &mut S {
                vars: &mut sprite.vars,
                struct_lists,
                callsites,
                funcs: &sprite.funcs,
                func: Some(func),
            },
        );
    }
    for event in &mut sprite.events {
        visit_stmts(
            &mut event.body,
            &mut S {
                vars: &mut sprite.vars,
                struct_lists,
                callsites,
                funcs: &sprite.funcs,
                func: None,
            },
        );
        match &mut event.kind {
            EventKind::OnLoudnessGt { value } | EventKind::OnTimerGt { value } => {
                visit_expr(
                    value,
                    &mut vec![],
                    &mut S {
                        vars: &mut sprite.vars,
                        struct_lists,
                        callsites,
                        funcs: &sprite.funcs,
                        func: None,
                    },
                );
            }
            _ => {}
        }
    }
    if *callsites != old_callsites {
        visit_sprite(sprite, callsites, struct_lists);
    }
}

fn visit_stmts(stmts: &mut Vec<Stmt>, s: &mut S) {
    let mut i = 0;
    while i < stmts.len() {
        let before = visit_stmt(&mut stmts[i], s);
        for stmt in before {
            stmts.insert(i, stmt);
            i += 1;
        }
        i += 1;
    }
}

fn visit_stmt(stmt: &mut Stmt, s: &mut S) -> Vec<Stmt> {
    let mut replace = None;
    let mut before = vec![];
    match stmt {
        Stmt::Repeat { times, body } => {
            visit_expr(times, &mut before, s);
            visit_stmts(body, s);
        }
        Stmt::Forever { body, span: _ } => visit_stmts(body, s),
        Stmt::Branch {
            cond,
            if_body,
            else_body,
        } => {
            visit_expr(cond, &mut before, s);
            visit_stmts(if_body, s);
            visit_stmts(else_body, s);
        }
        Stmt::Until { cond, body } => {
            let mut cond_callsites = vec![];
            visit_expr(cond, &mut cond_callsites, s);
            visit_stmts(body, s);
            before.extend(cond_callsites.iter().cloned());
            body.extend(cond_callsites);
        }
        Stmt::SetVar {
            name,
            value,
            type_: _,
            is_local: _,
            is_cloud: _,
        } => {
            visit_expr(value, &mut before, s);
            hoist_struct_self_references(name, value, None, &mut before, s);
        }
        Stmt::ChangeVar { name: _, value } => {
            visit_expr(value, &mut before, s);
        }
        Stmt::Show(_name) => {}
        Stmt::Hide(_name) => {}
        Stmt::AddToList { name, value } => {
            visit_expr(value, &mut before, s);
            hoist_struct_self_references(name, value, None, &mut before, s);
        }
        Stmt::DeleteList(_name) => {}
        Stmt::DeleteListIndex { name, index } => {
            visit_expr(index, &mut before, s);
            hoist_struct_list_index(name, index, false, &mut before, s);
        }
        Stmt::InsertAtList { name, index, value } => {
            visit_expr(index, &mut before, s);
            visit_expr(value, &mut before, s);
            hoist_struct_self_references(name, value, Some(index), &mut before, s);
            hoist_struct_list_index(name, index, true, &mut before, s);
        }
        Stmt::SetListIndex { name, index, value } => {
            visit_expr(index, &mut before, s);
            visit_expr(value, &mut before, s);
            hoist_struct_self_references(name, value, Some(index), &mut before, s);
            hoist_struct_list_index(name, index, false, &mut before, s);
        }
        Stmt::Block {
            block: _,
            span: _,
            args,
            kwargs,
        } => {
            for arg in args {
                visit_expr(arg, &mut before, s);
            }
            for (_, arg) in kwargs.values_mut() {
                visit_expr(arg, &mut before, s);
            }
        }
        Stmt::ProcCall {
            name: _,
            span: _,
            args,
            kwargs,
        } => {
            for arg in args {
                visit_expr(arg, &mut before, s);
            }
            for (_, arg) in kwargs.values_mut() {
                visit_expr(arg, &mut before, s);
            }
        }
        Stmt::FuncCall {
            name: _,
            span: _,
            args,
            kwargs,
        } => {
            for arg in args {
                visit_expr(arg, &mut before, s);
            }
            for (_, arg) in kwargs.values_mut() {
                visit_expr(arg, &mut before, s);
            }
        }
        Stmt::Return { value, visited } => {
            if !*visited {
                *visited = true;
                if let Some(func) = s.func {
                    // generated statements keep the location of the `return` value
                    let span = value.span();
                    before.push(Stmt::SetVar {
                        name: Name::Name {
                            name: format!("{}:return", func.name).into(),
                            span: span.clone(),
                        },
                        value: value.clone(),
                        type_: Type::Value,
                        is_local: false,
                        is_cloud: false,
                    });
                    replace = Some(Stmt::Return {
                        value: Expr::Value {
                            value: Value::from(0.0),
                            span,
                        }
                        .into(),
                        visited: true,
                    })
                }
            }
        }
    }
    if let Some(replace) = replace {
        *stmt = replace;
    }
    before
}

fn visit_expr(expr: &mut Expr, before: &mut Vec<Stmt>, s: &mut S) {
    let replace: Option<Expr> = match expr {
        Expr::Value { value: _, span: _ } => None,
        Expr::Name(_) => None,
        Expr::Dot {
            lhs,
            rhs: _,
            rhs_span: _,
        } => {
            visit_expr(lhs, before, s);
            None
        }
        Expr::Arg(_) => None,
        Expr::Repr {
            repr: _,
            span: _,
            args,
        } => {
            for arg in args {
                visit_expr(arg, before, s);
            }
            None
        }
        Expr::FuncCall {
            name,
            span,
            args,
            kwargs,
        } => {
            if let Some(func) = s.funcs.get(name) {
                *s.callsites += 1;
                before.push(Stmt::FuncCall {
                    name: name.clone(),
                    span: span.clone(),
                    args: args.clone(),
                    kwargs: kwargs.clone(),
                });
                let callsite = Name::Name {
                    name: format!("@{}", *s.callsites).into(),
                    span: span.clone(),
                };
                s.vars.insert(
                    callsite.basename().clone(),
                    Var {
                        name: callsite.basename().clone(),
                        span: callsite.basespan().clone(),
                        type_: func.type_.clone(),
                        default: None,
                        is_cloud: false,
                        is_used: true,
                    },
                );
                before.push(Stmt::SetVar {
                    name: callsite.clone(),
                    value: Box::new(Expr::Name(Name::Name {
                        name: format!("{}:return", name).into(),
                        span: span.clone(),
                    })),
                    type_: Type::Value,
                    is_local: false,
                    is_cloud: false,
                });
                Some(Expr::Name(callsite))
            } else {
                None
            }
        }
        Expr::UnOp {
            op: _,
            span: _,
            opr,
        } => {
            visit_expr(opr, before, s);
            None
        }
        Expr::BinOp {
            op: _,
            span: _,
            lhs,
            rhs,
        } => {
            visit_expr(lhs, before, s);
            visit_expr(rhs, before, s);
            None
        }
        Expr::StructLiteral {
            name: _,
            span: _,
            fields,
        } => {
            for field in fields {
                visit_expr(&mut field.value, before, s);
            }
            None
        }
        Expr::Property {
            object,
            property: _,
            span: _,
        } => {
            visit_expr(object, before, s);
            None
        }
        Expr::Ternary { .. } => unreachable!(),
    };
    if let Some(replace) = replace {
        *expr = replace;
    }
}

/// A struct assignment from a struct literal (`a = P { x: a.y, y: a.x };`, also for struct list
/// items) is lowered to one assignment per field, in field order. Each field value that reads
/// the assigned variable or list (and a list index that does) would then see fields that were
/// already overwritten. Evaluate those values into temporaries first, so the assignment is
/// atomic (GBS-003).
fn hoist_struct_self_references(
    target: &Name,
    value: &mut Expr,
    index: Option<&mut Expr>,
    before: &mut Vec<Stmt>,
    s: &mut S,
) {
    if target.fieldname().is_some() {
        return;
    }
    let Expr::StructLiteral { fields, .. } = value else {
        return;
    };
    if fields.len() < 2 {
        return;
    }
    let basename = target.basename().clone();
    let hoist = |expr: &mut Expr, before: &mut Vec<Stmt>, s: &mut S| {
        if !mentions(expr, &basename) {
            return;
        }
        *s.callsites += 1;
        let span = expr.span();
        let temp = Name::Name {
            name: format!("@{}", *s.callsites).into(),
            span: span.clone(),
        };
        s.vars.insert(
            temp.basename().clone(),
            Var {
                name: temp.basename().clone(),
                span: span.clone(),
                type_: Type::Value,
                default: None,
                is_cloud: false,
                is_used: true,
            },
        );
        before.push(Stmt::SetVar {
            name: temp.clone(),
            value: Box::new(expr.clone()),
            type_: Type::Value,
            is_local: false,
            is_cloud: false,
        });
        *expr = Expr::Name(temp);
    };
    for field in fields.iter_mut() {
        hoist(&mut field.value, before, s);
    }
    if let Some(index) = index {
        hoist(index, before, s);
    }
}

/// Evaluate `expr` into a new temporary variable before the statement; `expr` becomes the
/// temporary.
fn hoist_into_temp(expr: &mut Expr, before: &mut Vec<Stmt>, s: &mut S) {
    *s.callsites += 1;
    let span = expr.span();
    let temp = Name::Name {
        name: format!("@{}", *s.callsites).into(),
        span: span.clone(),
    };
    s.vars.insert(
        temp.basename().clone(),
        Var {
            name: temp.basename().clone(),
            span: span.clone(),
            type_: Type::Value,
            default: None,
            is_cloud: false,
            is_used: true,
        },
    );
    before.push(Stmt::SetVar {
        name: temp.clone(),
        value: Box::new(expr.clone()),
        type_: Type::Value,
        is_local: false,
        is_cloud: false,
    });
    *expr = Expr::Name(temp);
}

/// An item operation on a struct list (`delete L[i]`, `insert v at L[i]`, `L[i] = v`) is one
/// list operation per field, each with its own copy of the index. An index that is not a
/// literal, variable or argument is evaluated once into a temporary, and the literal index
/// "random"/"any" becomes `random(1, length L)` (`+ 1` for insert; the same
/// single draw Scratch makes) in a temporary, so all fields address the same item (GBS-038).
/// A variable that holds the text "random" still picks one item per field.
fn hoist_struct_list_index(
    name: &Name,
    index: &mut Expr,
    is_insert: bool,
    before: &mut Vec<Stmt>,
    s: &mut S,
) {
    if name.fieldname().is_some() {
        return;
    }
    let Some(fields) = s.struct_lists.get(name.basename()) else {
        return;
    };
    if fields.is_empty() {
        return;
    }
    let span = index.span();
    match index {
        Expr::Value {
            value: Value::String(text),
            ..
        } if text == "random" || text == "any" => {
            // `length L` of a struct list is the length of its first field list
            let list = Expr::Name(Name::Name {
                name: name.basename().clone(),
                span: name.basespan().clone(),
            });
            let mut high = UnOp::Length.to_expr(span.clone(), list);
            if is_insert {
                high = BinOp::Add.to_expr(span.clone(), high, Value::from(1.0).to_expr(span.clone()));
            }
            *index = Expr::Repr {
                repr: Repr::Random,
                span: span.clone(),
                args: vec![Value::from(1.0).to_expr(span.clone()), high],
            };
            hoist_into_temp(index, before, s);
        }
        Expr::Value { .. } | Expr::Name(_) | Expr::Arg(_) => {}
        _ => hoist_into_temp(index, before, s),
    }
}

/// Whether `expr` reads the variable or list called `basename` (or one of its fields).
fn mentions(expr: &Expr, basename: &SmolStr) -> bool {
    match expr {
        Expr::Value { .. } | Expr::Arg(_) => false,
        Expr::Name(name) => name.basename() == basename,
        Expr::Dot { lhs, .. } => mentions(lhs, basename),
        Expr::Repr { args, .. } => args.iter().any(|arg| mentions(arg, basename)),
        Expr::FuncCall { args, kwargs, .. } => {
            args.iter().any(|arg| mentions(arg, basename))
                || kwargs.values().any(|(_, arg)| mentions(arg, basename))
        }
        Expr::UnOp { opr, .. } => mentions(opr, basename),
        Expr::BinOp { lhs, rhs, .. } => mentions(lhs, basename) || mentions(rhs, basename),
        Expr::StructLiteral { fields, .. } => {
            fields.iter().any(|field| mentions(&field.value, basename))
        }
        Expr::Property { object, .. } => mentions(object, basename),
        Expr::Ternary {
            condition,
            tvalue,
            fvalue,
        } => {
            mentions(condition, basename)
                || mentions(tvalue, basename)
                || mentions(fvalue, basename)
        }
    }
}
