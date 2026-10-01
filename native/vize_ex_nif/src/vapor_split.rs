use rustler::{Atom, Encoder, Env, Term};
use vize_atelier_vapor::ir::*;

use crate::atoms;
use crate::html_inject::{
    build_elem_to_tag, inject_attr, inject_before_close, parse_tag_tree,
    replace_first_space_in_content, replace_range, replace_text_node, TagEntry,
};
use crate::ir_encoders::{encode_directive_ir_node, encode_ir_prop, encode_set_event_ir_node};
use crate::ir_encoding::encode_simple_expr;
use crate::term_encoding::{
    EncodedComponentSlot, EncodedForSlot, EncodedIfSlot, EncodedSplitBinding, EncodedSplitBlock,
    EncodedValueSlot, EncodedValuesSlot,
};

fn encode_slot_values<'a>(env: Env<'a>, kind: Atom, values: Vec<Term<'a>>) -> Term<'a> {
    EncodedValuesSlot { kind, values }.encode(env)
}

fn encode_slot_value<'a>(
    env: Env<'a>,
    kind: Atom,
    expr: &vize_atelier_core::SimpleExpressionNode,
) -> Term<'a> {
    EncodedValueSlot {
        kind,
        value: encode_simple_expr(env, expr),
    }
    .encode(env)
}

fn split_block<'a, 'b>(
    env: Env<'a>,
    block: &'b BlockIRNode<'b>,
    ir: &'b RootIRNode<'b>,
    source: &str,
) -> EncodedSplitBlock<'a> {
    let (statics, slots, bindings) = process_block(env, block, ir, source);
    EncodedSplitBlock {
        statics,
        slots,
        bindings,
    }
}

fn encode_slot_if_split<'a, 'b>(
    env: Env<'a>,
    if_node: &'b IfIRNode<'b>,
    ir: &'b RootIRNode<'b>,
    source: &str,
) -> Term<'a> {
    let negative = match &if_node.negative {
        Some(NegativeBranch::Block(block)) => Some(split_block(env, block, ir, source).encode(env)),
        Some(NegativeBranch::If(nested)) => Some(encode_slot_if_split(env, nested, ir, source)),
        None => None,
    };

    EncodedIfSlot {
        kind: atoms::if_node(),
        condition: encode_simple_expr(env, &if_node.condition),
        positive: split_block(env, &if_node.positive, ir, source),
        negative,
    }
    .encode(env)
}

fn encode_slot_for_split<'a, 'b>(
    env: Env<'a>,
    for_node: &'b ForIRNode<'b>,
    ir: &'b RootIRNode<'b>,
    source: &str,
) -> Term<'a> {
    EncodedForSlot {
        kind: atoms::for_node(),
        source: encode_simple_expr(env, &for_node.source),
        value: for_node
            .value
            .as_ref()
            .map(|value| encode_simple_expr(env, value)),
        key_prop: for_node
            .key_prop
            .as_ref()
            .map(|key_prop| encode_simple_expr(env, key_prop)),
        render: split_block(env, &for_node.render, ir, source),
    }
    .encode(env)
}

fn encode_slot_component<'a>(env: Env<'a>, node: &CreateComponentIRNode) -> Term<'a> {
    let kind = match node.kind {
        ComponentKind::Regular => atoms::regular(),
        ComponentKind::Teleport => atoms::teleport(),
        ComponentKind::KeepAlive => atoms::keep_alive(),
        ComponentKind::Suspense => atoms::suspense(),
        ComponentKind::Transition => atoms::transition(),
        ComponentKind::TransitionGroup => atoms::transition_group(),
        ComponentKind::Dynamic => atoms::dynamic(),
    };

    EncodedComponentSlot {
        kind: atoms::create_component(),
        tag: node.tag.to_string(),
        props: node
            .props
            .iter()
            .map(|prop| encode_ir_prop(env, prop))
            .collect(),
        value: kind,
    }
    .encode(env)
}

const SLOT_MARKER_PREFIX: &str = "\0VIZE_SLOT_";
const BINDING_MARKER_PREFIX: &str = "\0VIZE_BINDING_";
const SLOT_MARKER_SUFFIX: char = '\0';

struct SlotMarker<'a> {
    term: Option<Term<'a>>,
    source_offset: u32,
}

/// The text node Vapor bakes into its template for a `setText` target: static
/// parts verbatim and a single space in place of each dynamic value.
fn text_node_template<'b>(
    values: impl IntoIterator<Item = &'b vize_atelier_core::SimpleExpressionNode<'b>>,
) -> String {
    values
        .into_iter()
        .map(|value| if value.is_static { value.content } else { " " })
        .collect()
}

/// Only a `Node` anchor points at a `<!---->` placeholder; `Index` appends.
fn placeholder_anchor(anchor: Option<InsertionAnchor>) -> Option<usize> {
    match anchor {
        Some(InsertionAnchor::Node(id)) => Some(id),
        Some(InsertionAnchor::Index(_)) | None => None,
    }
}

fn slot_marker(index: usize) -> String {
    format!("{SLOT_MARKER_PREFIX}{index}{SLOT_MARKER_SUFFIX}")
}

// Marks where an element's start tag ends, for an event or v-model the split
// reports as a binding instead of rendering.
fn push_binding_marker<'a>(
    bindings: &mut Vec<(Atom, Term<'a>)>,
    kind: Atom,
    node: Term<'a>,
) -> String {
    let index = bindings.len();
    bindings.push((kind, node));
    format!("{BINDING_MARKER_PREFIX}{index}{SLOT_MARKER_SUFFIX}")
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

fn source_tag_at_offset(tags: &[TagEntry], offset: u32) -> Option<usize> {
    let offset = offset as usize;
    tags.iter()
        .filter(|tag| tag.open_start <= offset && offset < tag.open_end)
        .max_by_key(|tag| tag.open_start)
        .map(|tag| tag.pos)
}

fn structural_source_tags(
    block: &BlockIRNode<'_>,
    source_tags: &[TagEntry],
) -> std::collections::HashSet<usize> {
    let roots: std::collections::HashSet<usize> = block
        .operation
        .iter()
        .filter_map(|operation| match operation {
            OperationNode::If(node) => {
                source_tag_at_offset(source_tags, node.condition.loc.span.start)
            }
            OperationNode::For(node) => {
                source_tag_at_offset(source_tags, node.source.loc.span.start)
            }
            _ => None,
        })
        .collect();

    source_tags
        .iter()
        .filter(|tag| {
            let mut current = Some(tag.pos);
            while let Some(pos) = current {
                if roots.contains(&pos) {
                    return true;
                }
                current = source_tags.get(pos).and_then(|entry| entry.parent);
            }
            false
        })
        .map(|tag| tag.pos)
        .collect()
}

fn align_tag_source_offsets(
    rendered_tags: &[TagEntry],
    source: &str,
    block: &BlockIRNode<'_>,
) -> Vec<Option<u32>> {
    let source_tags = parse_tag_tree(source);
    let excluded = structural_source_tags(block, &source_tags);
    let mut cursor = 0usize;

    rendered_tags
        .iter()
        .map(|rendered| {
            if rendered.is_anchor() {
                return None;
            }
            let match_pos = source_tags[cursor..]
                .iter()
                .position(|source_tag| {
                    !excluded.contains(&source_tag.pos)
                        && !source_tag.is_anchor()
                        && source_tag.tag.eq_ignore_ascii_case(&rendered.tag)
                })
                .map(|relative| cursor + relative);

            match_pos.map(|pos| {
                cursor = pos + 1;
                source_tags[pos].open_start as u32
            })
        })
        .collect()
}

fn inject_structural_marker(
    html: &mut String,
    tags: &mut [TagEntry],
    elem_to_tag: &std::collections::HashMap<usize, usize>,
    insertion: (Option<usize>, Option<usize>),
    marker: &str,
    source_position: (&[Option<u32>], u32),
    slots: &[SlotMarker<'_>],
) {
    let (parent, anchor) = insertion;
    let (tag_source_offsets, source_offset) = source_position;
    if let Some(anchor_pos) = anchor.and_then(|id| elem_to_tag.get(&id)).copied() {
        if let Some(entry) = tags.get(anchor_pos) {
            // The marker takes the place of Vapor's own anchor comment.
            let anchor_len = if entry.is_anchor() {
                entry.open_end - entry.open_start
            } else {
                0
            };
            replace_range(html, tags, entry.open_start, anchor_len, marker);
            return;
        }
    }

    let bounds = parent
        .and_then(|id| elem_to_tag.get(&id))
        .and_then(|&tag_pos| tags.get(tag_pos))
        .map(|entry| (entry.open_end, entry.close_start.unwrap_or(entry.open_end)));

    let next_tag_position = tags
        .iter()
        .zip(tag_source_offsets)
        .filter(|(tag, offset)| {
            offset.is_some_and(|offset| offset > source_offset)
                && bounds
                    .map(|(start, end)| tag.open_start >= start && tag.open_start <= end)
                    .unwrap_or(true)
        })
        .map(|(tag, _)| tag.open_start)
        .min();

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

    if let Some(position) = [next_tag_position, next_marker_position]
        .into_iter()
        .flatten()
        .min()
    {
        replace_range(html, tags, position, 0, marker);
    } else if let Some(parent_id) = parent {
        if let Some(&tag_pos) = elem_to_tag.get(&parent_id) {
            inject_before_close(html, tags, tag_pos, marker);
        }
    } else {
        html.push_str(marker);
    }
}

// Splits the HTML at slot markers into statics, and turns binding markers into
// `{static_index, byte_offset}` positions in those statics.
fn split_on_markers<'a>(
    html: &str,
    mut slots: Vec<SlotMarker<'a>>,
    binding_nodes: Vec<(Atom, Term<'a>)>,
) -> (Vec<String>, Vec<Term<'a>>, Vec<EncodedSplitBinding<'a>>) {
    let unsplit = || (vec![html.to_string()], Vec::new(), Vec::new());
    let mut statics = Vec::new();
    let mut ordered_slots = Vec::new();
    let mut bindings: Vec<(usize, (usize, usize))> = Vec::new();
    let mut current = String::new();
    let mut rest = html;

    loop {
        let next_slot = rest
            .find(SLOT_MARKER_PREFIX)
            .map(|p| (p, SLOT_MARKER_PREFIX));
        let next_binding = rest
            .find(BINDING_MARKER_PREFIX)
            .map(|p| (p, BINDING_MARKER_PREFIX));

        let Some((position, prefix)) = [next_slot, next_binding]
            .into_iter()
            .flatten()
            .min_by_key(|(position, _)| *position)
        else {
            break;
        };

        current.push_str(&rest[..position]);

        let marker_body = &rest[position + prefix.len()..];
        let Some(suffix_position) = marker_body.find(SLOT_MARKER_SUFFIX) else {
            return unsplit();
        };
        let Ok(index) = marker_body[..suffix_position].parse::<usize>() else {
            return unsplit();
        };

        if prefix == SLOT_MARKER_PREFIX {
            let Some(slot) = slots.get_mut(index).and_then(|slot| slot.term.take()) else {
                return unsplit();
            };
            ordered_slots.push(slot);
            statics.push(std::mem::take(&mut current));
        } else {
            bindings.push((index, (statics.len(), current.len())));
        }

        rest = &marker_body[suffix_position + SLOT_MARKER_SUFFIX.len_utf8()..];
    }

    current.push_str(rest);
    statics.push(current);

    let bindings = bindings
        .into_iter()
        .map(|(index, at)| {
            let (kind, node) = binding_nodes[index];
            EncodedSplitBinding { kind, node, at }
        })
        .collect();

    (statics, ordered_slots, bindings)
}

pub(crate) fn process_block<'a, 'b>(
    env: Env<'a>,
    block: &'b BlockIRNode<'b>,
    ir: &'b RootIRNode<'b>,
    source: &str,
) -> (Vec<String>, Vec<Term<'a>>, Vec<EncodedSplitBinding<'a>>) {
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
    let tag_source_offsets = align_tag_source_offsets(&tags, source, block);
    let mut elem_to_tag = build_elem_to_tag(&block.returns, &block.operation, &tags);
    let mut slots: Vec<SlotMarker<'a>> = Vec::new();

    let mut binding_nodes: Vec<(Atom, Term<'a>)> = Vec::new();

    for op in &block.operation {
        if let OperationNode::SetEvent(event) = op {
            if let Some(&tag_pos) = elem_to_tag.get(&event.element) {
                let marker = push_binding_marker(
                    &mut binding_nodes,
                    atoms::set_event(),
                    encode_set_event_ir_node(env, event),
                );
                inject_attr(&mut html, &mut tags, tag_pos, &marker);
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
            let slot = encode_slot_values(env, atoms::set_prop(), values);
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
                            let slot = encode_slot_value(env, atoms::v_show(), simple);
                            let marker = push_slot_marker(&mut slots, slot, dir.dir.loc.span.start);
                            let attr = format!(" style=\"{marker}\"");
                            inject_attr(&mut html, &mut tags, tag_pos, &attr);
                        }
                    }
                    "model" => {
                        if let Some(vize_atelier_core::ExpressionNode::Simple(simple)) =
                            &dir.dir.exp
                        {
                            let slot = encode_slot_value(env, atoms::v_model(), simple);
                            let marker = push_slot_marker(&mut slots, slot, dir.dir.loc.span.start);
                            let attr = format!(" value=\"{marker}\"");
                            inject_attr(&mut html, &mut tags, tag_pos, &attr);
                            let marker = push_binding_marker(
                                &mut binding_nodes,
                                atoms::directive(),
                                encode_directive_ir_node(env, dir),
                            );
                            inject_attr(&mut html, &mut tags, tag_pos, &marker);
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
            let slot = encode_slot_values(env, atoms::set_text(), values);
            let source_offset = text
                .values
                .first()
                .map(|value| value.loc.span.start)
                .unwrap_or(u32::MAX);
            let marker = push_slot_marker(&mut slots, slot, source_offset);
            let template_text = text_node_template(text.values.iter().map(|value| &**value));
            if !replace_text_node(&mut html, &mut tags, tag_pos, &template_text, &marker) {
                replace_first_space_in_content(&mut html, &mut tags, tag_pos, &marker);
            }
        }
    }

    for html_effect in &html_effects {
        if let Some(&tag_pos) = elem_to_tag.get(&html_effect.element) {
            let slot = encode_slot_value(env, atoms::set_html(), &html_effect.value);
            let marker = push_slot_marker(&mut slots, slot, html_effect.value.loc.span.start);
            // `v-html` owns the element's content, which the template leaves empty.
            if !replace_first_space_in_content(&mut html, &mut tags, tag_pos, &marker) {
                inject_before_close(&mut html, &mut tags, tag_pos, &marker);
            }
        }
    }

    for operation in &block.operation {
        match operation {
            OperationNode::If(if_node) => {
                let source_offset = if_node.condition.loc.span.start;
                let slot = encode_slot_if_split(env, if_node, ir, source);
                let marker = push_slot_marker(&mut slots, slot, source_offset);
                inject_structural_marker(
                    &mut html,
                    &mut tags,
                    &elem_to_tag,
                    (if_node.parent, placeholder_anchor(if_node.anchor)),
                    &marker,
                    (&tag_source_offsets, source_offset),
                    &slots,
                );
            }
            OperationNode::For(for_node) => {
                let source_offset = for_node.source.loc.span.start;
                let slot = encode_slot_for_split(env, for_node, ir, source);
                let marker = push_slot_marker(&mut slots, slot, source_offset);
                inject_structural_marker(
                    &mut html,
                    &mut tags,
                    &elem_to_tag,
                    (for_node.parent, placeholder_anchor(for_node.anchor)),
                    &marker,
                    (&tag_source_offsets, source_offset),
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
                    (component.parent, placeholder_anchor(component.anchor)),
                    &marker,
                    (&tag_source_offsets, source_offset),
                    &slots,
                );
            }
            _ => {}
        }
    }

    split_on_markers(&html, slots, binding_nodes)
}
