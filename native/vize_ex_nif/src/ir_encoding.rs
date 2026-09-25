use rustler::{Encoder, Env, Term};
use vize_atelier_vapor::ir::{IRProp, InsertionAnchor, MergedPropsSource};

use crate::atoms;

pub(crate) fn encode_simple_expr<'a>(
    env: Env<'a>,
    expr: &vize_atelier_core::SimpleExpressionNode,
) -> Term<'a> {
    if expr.is_static {
        rustler::types::tuple::make_tuple(
            env,
            &[atoms::static_().encode(env), expr.content.encode(env)],
        )
    } else {
        expr.content.encode(env)
    }
}

/// A placeholder anchor keeps its node id; an append position becomes `{:index, n}`.
pub(crate) fn encode_insertion_anchor<'a>(
    env: Env<'a>,
    anchor: Option<InsertionAnchor>,
) -> Term<'a> {
    match anchor {
        Some(InsertionAnchor::Node(id)) => id.encode(env),
        Some(InsertionAnchor::Index(index)) => {
            rustler::types::tuple::make_tuple(env, &[atoms::index().encode(env), index.encode(env)])
        }
        None => rustler::types::atom::nil().encode(env),
    }
}

pub(crate) fn encode_merged_props_source<'a>(env: Env<'a>, source: &MergedPropsSource) -> Term<'a> {
    match source {
        MergedPropsSource::Object(expr) => term_map!(env, {
            atoms::kind() => atoms::object(),
            atoms::value() => encode_simple_expr(env, expr),
        }),
        MergedPropsSource::Group(props) => {
            let props: Vec<Term<'a>> = props.iter().map(|prop| encode_ir_prop(env, prop)).collect();
            term_map!(env, {
                atoms::kind() => atoms::group(),
                atoms::props() => props,
            })
        }
    }
}

pub(crate) fn encode_ir_prop<'a>(env: Env<'a>, prop: &IRProp) -> Term<'a> {
    let values: Vec<Term<'a>> = prop
        .values
        .iter()
        .map(|value| encode_simple_expr(env, value))
        .collect();

    term_map!(env, {
        atoms::key() => encode_simple_expr(env, &prop.key),
        atoms::values() => values,
        atoms::is_component() => prop.is_component,
    })
}
