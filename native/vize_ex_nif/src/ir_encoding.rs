use rustler::{Encoder, Env, Term};
use vize_atelier_vapor::ir::InsertionAnchor;

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
pub(crate) fn encode_insertion_anchor<'a>(env: Env<'a>, anchor: &InsertionAnchor) -> Term<'a> {
    match anchor {
        InsertionAnchor::Node(id) => id.encode(env),
        InsertionAnchor::Index(index) => {
            rustler::types::tuple::make_tuple(env, &[atoms::index().encode(env), index.encode(env)])
        }
    }
}

/// A directive's expression as source text: simple expressions keep their
/// static marker, compound expressions are joined.
pub(crate) fn encode_directive_expression<'a>(
    env: Env<'a>,
    directive: &vize_atelier_core::DirectiveNode,
) -> Term<'a> {
    match &directive.exp {
        Some(vize_atelier_core::ExpressionNode::Simple(simple)) => encode_simple_expr(env, simple),
        Some(vize_atelier_core::ExpressionNode::Compound(compound)) => {
            let content: std::string::String = compound
                .children
                .iter()
                .map(|child| match child {
                    vize_atelier_core::CompoundExpressionChild::Simple(simple) => {
                        simple.content.to_string()
                    }
                    vize_atelier_core::CompoundExpressionChild::String(string) => {
                        string.to_string()
                    }
                    _ => std::string::String::new(),
                })
                .collect();
            content.as_str().encode(env)
        }
        None => rustler::types::atom::nil().encode(env),
    }
}
