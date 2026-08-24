use rustler::{Encoder, Env, Term};
use vize_atelier_vapor::ir::*;

use crate::atoms;
use crate::html_inject::{
    build_elem_to_tag, inject_attr, inject_before_close, parse_tag_tree,
    replace_first_space_in_content, replace_range, TagEntry,
};
use crate::ir_encoding::{encode_ir_prop, encode_simple_expr};
use crate::term_encoding::nil_term;

fn encode_slot_values<'a>(env: Env<'a>, kind: Term<'a>, values: Term<'a>) -> Term<'a> {
    term_map!(env, {
        atoms::kind() => kind,
        atoms::values() => values,
    })
}

fn encode_slot_value<'a>(
    env: Env<'a>,
    kind: Term<'a>,
    expr: &vize_atelier_core::SimpleExpressionNode,
) -> Term<'a> {
    term_map!(env, {
        atoms::kind() => kind,
        atoms::value() => encode_simple_expr(env, expr),
    })
}

fn encode_split_block<'a, 'b>(
    env: Env<'a>,
    block: &'b BlockIRNode<'b>,
    ir: &'b RootIRNode<'b>,
) -> Term<'a> {
    let (statics, slots) = process_block(env, block, ir);
    let statics_term: Vec<Term<'a>> = statics
        .iter()
        .map(|static_part| static_part.as_str().encode(env))
        .collect();

    term_map!(env, {
        atoms::statics() => statics_term,
        atoms::slots() => slots,
    })
}

fn encode_slot_if_split<'a, 'b>(
    env: Env<'a>,
    if_node: &'b IfIRNode<'b>,
    ir: &'b RootIRNode<'b>,
) -> Term<'a> {
    let negative = match &if_node.negative {
        Some(NegativeBranch::Block(block)) => encode_split_block(env, block, ir),
        Some(NegativeBranch::If(nested)) => encode_slot_if_split(env, nested, ir),
        None => nil_term(env),
    };

    term_map!(env, {
        atoms::kind() => atoms::if_node(),
        atoms::condition() => encode_simple_expr(env, &if_node.condition),
        atoms::positive() => encode_split_block(env, &if_node.positive, ir),
        atoms::negative() => negative,
    })
}

fn encode_slot_for_split<'a, 'b>(
    env: Env<'a>,
    for_node: &'b ForIRNode<'b>,
    ir: &'b RootIRNode<'b>,
) -> Term<'a> {
    term_map!(env, {
        atoms::kind() => atoms::for_node(),
        atoms::source() => encode_simple_expr(env, &for_node.source),
        atoms::value() => for_node
            .value
            .as_ref()
            .map(|value| encode_simple_expr(env, value))
            .unwrap_or_else(|| nil_term(env)),
        atoms::key_prop() => for_node
            .key_prop
            .as_ref()
            .map(|key_prop| encode_simple_expr(env, key_prop))
            .unwrap_or_else(|| nil_term(env)),
        atoms::render() => encode_split_block(env, &for_node.render, ir),
    })
}

fn encode_slot_component<'a>(env: Env<'a>, node: &CreateComponentIRNode) -> Term<'a> {
    let props: Vec<Term<'a>> = node
        .props
        .iter()
        .map(|prop| encode_ir_prop(env, prop))
        .collect();
    let kind_atom = match node.kind {
        ComponentKind::Regular => atoms::regular(),
        ComponentKind::Teleport => atoms::teleport(),
        ComponentKind::KeepAlive => atoms::keep_alive(),
        ComponentKind::Suspense => atoms::suspense(),
        ComponentKind::Dynamic => atoms::dynamic(),
    };

    term_map!(env, {
        atoms::kind() => atoms::create_component(),
        atoms::tag() => node.tag,
        atoms::props() => props,
        atoms::value() => kind_atom,
    })
}

const SLOT_MARKER_PREFIX: &str = "\0VIZE_SLOT_";
const SLOT_MARKER_SUFFIX: char = '\0';

struct SlotMarker<'a> {
    term: Option<Term<'a>>,
    source_offset: u32,
}

fn slot_marker(index: usize) -> String {
    format!("{SLOT_MARKER_PREFIX}{index}{SLOT_MARKER_SUFFIX}")
}

fn push_slot_marker<'a>(
    slots: &mut Vec<SlotMarker<'a>>,
    slot: Term<'a>,
    source_offset: u32,
) -> String {
    let index = slots.len();
    slots.push(SlotMarker {
        term: Some(slot),
        source_offset,
    });
    slot_marker(index)
}

fn inject_structural_marker(
    html: &mut String,
    tags: &mut [TagEntry],
    elem_to_tag: &std::collections::HashMap<usize, usize>,
    insertion: (Option<usize>, Option<usize>),
    marker: &str,
    source_offset: u32,
    slots: &[SlotMarker<'_>],
) {
    let (parent, anchor) = insertion;
    if let Some(anchor_pos) = anchor.and_then(|id| elem_to_tag.get(&id)).copied() {
        if let Some(entry) = tags.get(anchor_pos) {
            replace_range(html, tags, entry.open_start, 0, marker);
            return;
        }
    }

    let bounds = parent
        .and_then(|id| elem_to_tag.get(&id))
        .and_then(|&tag_pos| tags.get(tag_pos))
        .map(|entry| (entry.open_end, entry.close_start.unwrap_or(entry.open_end)));

    let next_marker_position = slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.source_offset > source_offset)
        .filter_map(|(index, _)| html.find(&slot_marker(index)))
        .filter(|position| {
            bounds
                .map(|(start, end)| *position >= start && *position <= end)
                .unwrap_or(true)
        })
        .min();

    if let Some(position) = next_marker_position {
        replace_range(html, tags, position, 0, marker);
    } else if let Some(parent_id) = parent {
        if let Some(&tag_pos) = elem_to_tag.get(&parent_id) {
            inject_before_close(html, tags, tag_pos, marker);
        }
    } else {
        html.push_str(marker);
    }
}

fn split_on_slot_markers<'a>(
    html: &str,
    mut slots: Vec<SlotMarker<'a>>,
) -> (Vec<String>, Vec<Term<'a>>) {
    let mut statics = Vec::new();
    let mut ordered_slots = Vec::new();
    let mut rest = html;

    while let Some(position) = rest.find(SLOT_MARKER_PREFIX) {
        statics.push(rest[..position].to_string());

        let marker_body = &rest[position + SLOT_MARKER_PREFIX.len()..];
        let Some(suffix_position) = marker_body.find(SLOT_MARKER_SUFFIX) else {
            return (vec![html.to_string()], Vec::new());
        };
        let Ok(index) = marker_body[..suffix_position].parse::<usize>() else {
            return (vec![html.to_string()], Vec::new());
        };
        let Some(slot) = slots.get_mut(index).and_then(|slot| slot.term.take()) else {
            return (vec![html.to_string()], Vec::new());
        };

        ordered_slots.push(slot);
        rest = &marker_body[suffix_position + SLOT_MARKER_SUFFIX.len_utf8()..];
    }

    statics.push(rest.to_string());
    (statics, ordered_slots)
}

pub(crate) fn process_block<'a, 'b>(
    env: Env<'a>,
    block: &'b BlockIRNode<'b>,
    ir: &'b RootIRNode<'b>,
) -> (Vec<String>, Vec<Term<'a>>) {
    let template_html: String = block
        .returns
        .iter()
        .map(|&elem_id| {
            let template_idx = ir
                .element_template_map
                .get(&elem_id)
                .copied()
                .unwrap_or(elem_id);
            ir.templates.get(template_idx).copied().unwrap_or("")
        })
        .collect();

    let mut html = template_html;
    let mut tags = parse_tag_tree(&html);
    let mut elem_to_tag = build_elem_to_tag(&block.returns, &block.operation, &tags);
    let mut slots: Vec<SlotMarker<'a>> = Vec::new();

    for op in &block.operation {
        if let OperationNode::SetEvent(event) = op {
            if let Some(&tag_pos) = elem_to_tag.get(&event.element) {
                let event_name = event.key.content;
                let handler = event
                    .value
                    .as_ref()
                    .map(|value| value.content)
                    .unwrap_or(event_name);
                let attr = format!(" phx-{}=\"{}\"", event_name, handler);
                inject_attr(&mut html, &mut tags, tag_pos, &attr);
            }
        }
    }

    let all_effects: Vec<_> = block
        .effect
        .iter()
        .flat_map(|effect| effect.operations.iter())
        .collect();

    let mut prop_effects = Vec::new();
    let mut text_effects = Vec::new();
    let mut html_effects = Vec::new();

    for operation in &all_effects {
        match operation {
            OperationNode::SetProp(prop) => prop_effects.push(prop),
            OperationNode::SetText(text) => text_effects.push(text),
            OperationNode::SetHtml(html_node) => html_effects.push(html_node),
            _ => {}
        }
    }

    for element in text_effects
        .iter()
        .map(|text| text.element)
        .chain(html_effects.iter().map(|html| html.element))
    {
        if elem_to_tag.contains_key(&element) {
            continue;
        }
        if let Some(parent_tag) = block
            .operation
            .iter()
            .find_map(|operation| match operation {
                OperationNode::ChildRef(child) if child.child_id == element => {
                    elem_to_tag.get(&child.parent_id).copied()
                }
                _ => None,
            })
        {
            elem_to_tag.insert(element, parent_tag);
        }
    }

    for prop in &prop_effects {
        if let Some(&tag_pos) = elem_to_tag.get(&prop.element) {
            let values: Vec<Term<'a>> = prop
                .prop
                .values
                .iter()
                .map(|value| encode_simple_expr(env, value))
                .collect();
            let slot = encode_slot_values(env, atoms::set_prop().encode(env), values.encode(env));
            let source_offset = prop
                .prop
                .values
                .first()
                .map(|value| value.loc.span.start)
                .unwrap_or(prop.prop.key.loc.span.start);
            let marker = push_slot_marker(&mut slots, slot, source_offset);
            let attr_name = prop.prop.key.content;
            let attr = format!(" {attr_name}=\"{marker}\"");
            inject_attr(&mut html, &mut tags, tag_pos, &attr);
        }
    }

    for op in &block.operation {
        if let OperationNode::Directive(dir) = op {
            if let Some(&tag_pos) = elem_to_tag.get(&dir.element) {
                match dir.name {
                    "vShow" => {
                        if let Some(vize_atelier_core::ExpressionNode::Simple(simple)) =
                            &dir.dir.exp
                        {
                            let slot = encode_slot_value(env, atoms::v_show().encode(env), simple);
                            let marker = push_slot_marker(&mut slots, slot, dir.dir.loc.span.start);
                            let attr = format!(" style=\"{marker}\"");
                            inject_attr(&mut html, &mut tags, tag_pos, &attr);
                        }
                    }
                    "model" => {
                        if let Some(vize_atelier_core::ExpressionNode::Simple(simple)) =
                            &dir.dir.exp
                        {
                            let slot = encode_slot_value(env, atoms::v_model().encode(env), simple);
                            let marker = push_slot_marker(&mut slots, slot, dir.dir.loc.span.start);
                            let attr = format!(" value=\"{marker}\"");
                            inject_attr(&mut html, &mut tags, tag_pos, &attr);
                            let handler_name = format!("{}_changed", simple.content);
                            let change_attr = format!(" phx-change=\"{}\"", handler_name);
                            inject_attr(&mut html, &mut tags, tag_pos, &change_attr);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    for text in &text_effects {
        if let Some(&tag_pos) = elem_to_tag.get(&text.element) {
            let values: Vec<Term<'a>> = text
                .values
                .iter()
                .map(|value| encode_simple_expr(env, value))
                .collect();
            let slot = encode_slot_values(env, atoms::set_text().encode(env), values.encode(env));
            let source_offset = text
                .values
                .first()
                .map(|value| value.loc.span.start)
                .unwrap_or(u32::MAX);
            let marker = push_slot_marker(&mut slots, slot, source_offset);
            replace_first_space_in_content(&mut html, &mut tags, tag_pos, &marker);
        }
    }

    for html_effect in &html_effects {
        if let Some(&tag_pos) = elem_to_tag.get(&html_effect.element) {
            let slot = encode_slot_value(env, atoms::set_html().encode(env), &html_effect.value);
            let marker = push_slot_marker(&mut slots, slot, html_effect.value.loc.span.start);
            replace_first_space_in_content(&mut html, &mut tags, tag_pos, &marker);
        }
    }

    for operation in &block.operation {
        match operation {
            OperationNode::If(if_node) => {
                let source_offset = if_node.condition.loc.span.start;
                let slot = encode_slot_if_split(env, if_node, ir);
                let marker = push_slot_marker(&mut slots, slot, source_offset);
                inject_structural_marker(
                    &mut html,
                    &mut tags,
                    &elem_to_tag,
                    (if_node.parent, if_node.anchor),
                    &marker,
                    source_offset,
                    &slots,
                );
            }
            OperationNode::For(for_node) => {
                let source_offset = for_node.source.loc.span.start;
                let slot = encode_slot_for_split(env, for_node, ir);
                let marker = push_slot_marker(&mut slots, slot, source_offset);
                inject_structural_marker(
                    &mut html,
                    &mut tags,
                    &elem_to_tag,
                    (for_node.parent, for_node.anchor),
                    &marker,
                    source_offset,
                    &slots,
                );
            }
            OperationNode::CreateComponent(component) => {
                let source_offset = component
                    .props
                    .iter()
                    .flat_map(|prop| prop.values.iter())
                    .map(|value| value.loc.span.start)
                    .min()
                    .unwrap_or(u32::MAX);
                let slot = encode_slot_component(env, component);
                let marker = push_slot_marker(&mut slots, slot, source_offset);
                inject_structural_marker(
                    &mut html,
                    &mut tags,
                    &elem_to_tag,
                    (component.parent, component.anchor),
                    &marker,
                    source_offset,
                    &slots,
                );
            }
            _ => {}
        }
    }

    split_on_slot_markers(&html, slots)
}
