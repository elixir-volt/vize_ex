//! Splits a Vue template into static HTML and the dynamic slots between them.
//!
//! The template is lowered to Vize's L2 semantic IR, the op family Vize's own
//! SSR string plan is built from, and printed: static elements, attributes and
//! text become HTML, and each expression, structural directive, component and
//! slot outlet becomes a slot for the caller to render.

use std::collections::HashMap;

use rustler::{Encoder, Env, Term};
use vize_carton::line_index::LineIndex;
use vize_carton::{is_void_tag, Allocator};
use vize_davinci::diagnostic::{Diagnostic, Severity};
use vize_l1_to_l2::decode_ssr_static_text;
use vize_l1_to_l2::emit::decode_html_attribute_entities;
use vize_l1_to_l2::lower::TextPart;
use vize_l2::expr::{ExprRef, OpaqueReason};
use vize_l2::op::{
    Attribute, BindOp, BindingOp, ComponentOp, DynamicName, ElementOp, ForOp, IfOp, ModelOp, Op,
    Region, SlotOp,
};

use crate::atoms;
use crate::term_encoding::{
    EncodedAttrSlot, EncodedComponentSlot, EncodedEvent, EncodedForSlot, EncodedHtmlSlot,
    EncodedIfBranch, EncodedIfSlot, EncodedModelSlot, EncodedPosition, EncodedProp,
    EncodedRootAttrsSlot, EncodedSlotContent, EncodedSlotOutlet, EncodedSplitBinding,
    EncodedSplitBlock, EncodedSplitDiagnostic, EncodedSpreadSlot, EncodedTemplateSplit,
    EncodedTextSlot,
};

type Position = (usize, usize);

/// Splits `source`, a template's content. With `root_attrs`, the attributes of
/// a single root element come back as one `:root_attrs` slot. Errors are the
/// diagnostics that make the template unrenderable.
pub(crate) fn split_template<'a>(
    env: Env<'a>,
    source: &str,
    root_attrs: bool,
) -> Result<EncodedTemplateSplit<'a>, Vec<EncodedSplitDiagnostic>> {
    let allocator = Allocator::new();
    let (tree, errors) = vize_l1::parse(&allocator, source);
    let lowered = vize_l1_to_l2::lower(&allocator, &tree, &errors);

    let mut splitter = Splitter {
        env,
        lines: LineIndex::new(source),
        texts: lowered
            .texts
            .iter()
            .filter_map(|(_id, parts)| {
                Some((parts.parts.first()?.span.start, parts.parts.as_slice()))
            })
            .collect(),
        diagnostics: lowered
            .diagnostics
            .iter()
            .filter_map(|diagnostic| lowering_diagnostic(diagnostic, source))
            .collect(),
    };

    let mut block = Block::default();
    let root = if root_attrs {
        single_root(&lowered.root)
    } else {
        None
    };

    for op in lowered.root.ops.iter() {
        match (op, root) {
            (Op::Element(element), Some(root)) if std::ptr::eq(&**element, root) => {
                splitter.element(&mut block, element, true)
            }
            _ => splitter.op(&mut block, op),
        }
    }

    let errors: Vec<_> = splitter
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == atoms::error())
        .cloned()
        .collect();

    if !errors.is_empty() {
        return Err(errors);
    }

    let EncodedSplitBlock {
        statics,
        slots,
        bindings,
    } = block.finish();

    Ok(EncodedTemplateSplit {
        statics,
        slots,
        bindings,
        diagnostics: splitter.diagnostics,
    })
}

/// The template's only element, ignoring whitespace and comments around it.
fn single_root<'r, 'a>(root: &'r Region<'a>) -> Option<&'r ElementOp<'a>> {
    let mut elements = root.ops.iter().filter(|op| match op {
        Op::Text(text) => !text.content.trim().is_empty(),
        Op::Comment(_) => false,
        _ => true,
    });

    match (elements.next(), elements.next()) {
        (Some(Op::Element(element)), None) => Some(element),
        _ => None,
    }
}

/// Static HTML and the slots between its pieces, built in document order.
#[derive(Default)]
struct Block<'a> {
    statics: Vec<String>,
    current: String,
    slots: Vec<Term<'a>>,
    bindings: Vec<EncodedSplitBinding>,
}

impl<'a> Block<'a> {
    fn push(&mut self, html: &str) {
        self.current.push_str(html);
    }

    fn slot(&mut self, slot: Term<'a>) {
        self.statics.push(std::mem::take(&mut self.current));
        self.slots.push(slot);
    }

    /// Where the next byte goes: `{static_index, byte_offset}`.
    fn at(&self) -> (usize, usize) {
        (self.statics.len(), self.current.len())
    }

    fn finish(mut self) -> EncodedSplitBlock<'a> {
        self.statics.push(self.current);
        EncodedSplitBlock {
            statics: self.statics,
            slots: self.slots,
            bindings: self.bindings,
        }
    }
}

struct Splitter<'s, 'a> {
    env: Env<'a>,
    lines: LineIndex<'s>,
    /// Merged text runs such as `Hello {{ name }}!`, by where they start.
    texts: HashMap<u32, &'s [TextPart]>,
    diagnostics: Vec<EncodedSplitDiagnostic>,
}

impl<'s, 'a> Splitter<'s, 'a> {
    fn region(&mut self, region: &Region<'_>) -> EncodedSplitBlock<'a> {
        let mut block = Block::default();
        for op in region.ops.iter() {
            self.op(&mut block, op);
        }
        block.finish()
    }

    fn op(&mut self, block: &mut Block<'a>, op: &Op<'_>) {
        match op {
            Op::Element(element) => self.element(block, element, false),
            Op::Component(component) => {
                let slot = self.component(component);
                block.slot(slot);
            }
            Op::Text(text) => block.push(&escape_text(text.content)),
            Op::Interpolation(interpolation) => match interpolation.expression {
                ExprRef::Opaque(opaque) if opaque.reason == OpaqueReason::Compound => {
                    self.compound_text(block, interpolation.span.start, opaque.source)
                }
                expression => {
                    let slot = self.text_slot(expression);
                    block.slot(slot);
                }
            },
            Op::Comment(_) => {}
            Op::If(if_op) => {
                let slot = self.if_slot(if_op);
                block.slot(slot);
            }
            Op::For(for_op) => {
                let slot = self.for_slot(for_op);
                block.slot(slot);
            }
            Op::Slot(outlet) => {
                let slot = self.slot_outlet(outlet);
                block.slot(slot);
            }
        }
    }

    fn compound_text(&mut self, block: &mut Block<'a>, start: u32, source: &str) {
        let Some(parts) = self.texts.get(&start).copied() else {
            self.error(start, start + source.len() as u32, "can't split this text");
            return;
        };

        for part in parts {
            if part.dynamic {
                let slot = EncodedTextSlot {
                    kind: atoms::text(),
                    value: part.text.to_string(),
                    position: self.position(part.span.start),
                }
                .encode(self.env);
                block.slot(slot);
            } else {
                block.push(&escape_text(&part.text));
            }
        }
    }

    fn text_slot(&mut self, expression: ExprRef<'_>) -> Term<'a> {
        EncodedTextSlot {
            kind: atoms::text(),
            value: self.expression(expression),
            position: self.position(expression.span().start),
        }
        .encode(self.env)
    }

    fn element(&mut self, block: &mut Block<'a>, element: &ElementOp<'_>, root: bool) {
        block.push("<");
        block.push(element.tag);

        let bound = bound_names(&element.bindings);
        let show = element.bindings.iter().find_map(|binding| match binding {
            BindingOp::VueShow(show) => Some(show.value),
            _ => None,
        });

        if root {
            let slot = self.root_attrs(element, show);
            block.slot(slot);
        } else {
            self.attributes(block, element, &bound, show);
        }

        let mut content = None;

        for binding in element.bindings.iter() {
            match binding {
                BindingOp::On(on) => {
                    let binding = EncodedSplitBinding {
                        kind: atoms::on(),
                        name: on.name.and_then(static_name),
                        modifiers: on.modifiers.iter().map(ToString::to_string).collect(),
                        value: on.handler.map(|handler| self.expression(handler)),
                        at: block.at(),
                        position: self.position(on.span.start),
                    };
                    block.bindings.push(binding);
                }
                BindingOp::Model(model) => {
                    let value = self.expression(model.contract.read);
                    block.bindings.push(EncodedSplitBinding {
                        kind: atoms::model(),
                        name: model.argument.and_then(static_name),
                        modifiers: Vec::new(),
                        value: Some(value),
                        at: block.at(),
                        position: self.position(model.span.start),
                    });

                    let slot = self.model_slot(element, model);
                    if element.tag.eq_ignore_ascii_case("textarea") {
                        content = Some(slot);
                    } else if element.tag.eq_ignore_ascii_case("input") {
                        block.slot(slot);
                    } else {
                        self.warning(model.span.start, model.span.end, "v-model only renders its value on <input> and <textarea> on the server");
                    }
                }
                BindingOp::VueHtml(html) => {
                    content = html.value.map(|value| {
                        EncodedHtmlSlot {
                            kind: atoms::html(),
                            value: self.expression(value),
                            position: self.position(html.span.start),
                        }
                        .encode(self.env)
                    });
                }
                BindingOp::VueText(text) => {
                    content = text.value.map(|value| self.text_slot(value));
                }
                BindingOp::VueDirective(directive) => {
                    let message = format!(
                        "the custom directive v-{} doesn't run on the server",
                        directive.name
                    );
                    self.warning(directive.span.start, directive.span.end, &message);
                }
                BindingOp::SlotContent(slot) => {
                    self.error(
                        slot.span.start,
                        slot.span.end,
                        "slot content must be passed to a component",
                    );
                }
                BindingOp::VueSync(sync) => {
                    self.error(sync.span.start, sync.span.end, ".sync is Vue 2 syntax");
                }
                BindingOp::VueSlotScope(scope) => {
                    self.error(
                        scope.span.start,
                        scope.span.end,
                        "slot-scope is Vue 2 syntax",
                    );
                }
                BindingOp::Bind(_)
                | BindingOp::VueShow(_)
                | BindingOp::VueCssBind(_)
                | BindingOp::VueOnce(_)
                | BindingOp::VueMemo(_)
                | BindingOp::VueCloak(_) => {}
            }
        }

        block.push(">");

        if is_void_tag(element.tag) {
            return;
        }

        match content {
            Some(slot) => block.slot(slot),
            None => {
                for op in element.children.ops.iter() {
                    self.op(block, op);
                }
            }
        }

        block.push("</");
        block.push(element.tag);
        block.push(">");
    }

    /// Static attributes print as HTML. Each binding is an `:attr` slot, which
    /// takes in a static attribute of the same name, so `class="a"` and
    /// `:class="b"` render as one attribute.
    fn attributes(
        &mut self,
        block: &mut Block<'a>,
        element: &ElementOp<'_>,
        bound: &[&str],
        show: Option<ExprRef<'_>>,
    ) {
        let has_style = bound.contains(&"style");

        for attribute in element.attributes.iter() {
            let merged =
                bound.contains(&attribute.name) || (attribute.name == "style" && show.is_some());
            if !merged {
                block.push(&static_attribute(attribute));
            }
        }

        for binding in element.bindings.iter() {
            let BindingOp::Bind(bind) = binding else {
                continue;
            };

            let slot = match bind.name {
                Some(DynamicName::Static("key")) => continue,
                Some(DynamicName::Static(name)) => self.attr_slot(
                    Some(name),
                    None,
                    static_value(&element.attributes, name),
                    bind,
                    if name == "style" { show } else { None },
                ),
                Some(DynamicName::Dynamic(name)) => {
                    self.attr_slot(None, Some(name), None, bind, None)
                }
                None => match bind.value {
                    Some(value) => EncodedSpreadSlot {
                        kind: atoms::spread(),
                        value: self.expression(value),
                        position: self.position(bind.span.start),
                    }
                    .encode(self.env),
                    None => continue,
                },
            };
            block.slot(slot);
        }

        if let (Some(show), false) = (show, has_style) {
            let slot = EncodedAttrSlot {
                kind: atoms::attr(),
                name: Some("style".to_string()),
                name_value: None,
                r#static: static_value(&element.attributes, "style"),
                value: None,
                show: Some(self.expression(show)),
                position: self.position(show.span().start),
            }
            .encode(self.env);
            block.slot(slot);
        }
    }

    fn attr_slot(
        &mut self,
        name: Option<&str>,
        name_value: Option<ExprRef<'_>>,
        static_value: Option<String>,
        bind: &BindOp<'_>,
        show: Option<ExprRef<'_>>,
    ) -> Term<'a> {
        EncodedAttrSlot {
            kind: atoms::attr(),
            name: name.map(ToString::to_string),
            name_value: name_value.map(|name| self.expression(name)),
            r#static: static_value,
            value: bind.value.map(|value| self.expression(value)),
            show: show.map(|show| self.expression(show)),
            position: self.position(bind.span.start),
        }
        .encode(self.env)
    }

    fn root_attrs(&mut self, element: &ElementOp<'_>, show: Option<ExprRef<'_>>) -> Term<'a> {
        let props = self.props(&element.attributes, &element.bindings);
        EncodedRootAttrsSlot {
            kind: atoms::root_attrs(),
            props,
            show: show.map(|show| self.expression(show)),
            position: self.position(element.span.start),
        }
        .encode(self.env)
    }

    /// Static attributes and bindings as props, in authored order. A static
    /// attribute that a binding of the same name also sets is folded into it.
    fn props(
        &mut self,
        attributes: &[Attribute<'_>],
        bindings: &[BindingOp<'_>],
    ) -> Vec<EncodedProp> {
        let bound = bound_names(bindings);

        let statics = attributes
            .iter()
            .filter(|attribute| !bound.contains(&attribute.name))
            .map(|attribute| (attribute.span.start, static_prop(attribute)));

        let binds: Vec<_> = bindings
            .iter()
            .filter_map(|binding| match binding {
                BindingOp::Bind(bind) if !matches!(bind.name, Some(DynamicName::Static("key"))) => {
                    Some(bind)
                }
                _ => None,
            })
            .map(|bind| (bind.span.start, self.bind_prop(attributes, bind)))
            .collect();

        let mut props: Vec<_> = statics.chain(binds).collect();
        props.sort_by_key(|(start, _prop)| *start);
        props.into_iter().map(|(_start, prop)| prop).collect()
    }

    fn bind_prop(&mut self, attributes: &[Attribute<'_>], bind: &BindOp<'_>) -> EncodedProp {
        let (name, name_value) = match bind.name {
            Some(DynamicName::Static(name)) => (Some(name.to_string()), None),
            Some(DynamicName::Dynamic(name)) => (None, Some(self.expression(name))),
            None => (None, None),
        };

        EncodedProp {
            r#static: name
                .as_deref()
                .and_then(|name| static_value(attributes, name)),
            name,
            name_value,
            value: bind.value.map(|value| self.expression(value)),
        }
    }

    fn model_slot(&mut self, element: &ElementOp<'_>, model: &ModelOp<'_>) -> Term<'a> {
        EncodedModelSlot {
            kind: atoms::model(),
            tag: element.tag.to_string(),
            r#type: static_value(&element.attributes, "type"),
            static_value: static_value(&element.attributes, "value"),
            value: self.expression(model.contract.read),
            position: self.position(model.span.start),
        }
        .encode(self.env)
    }

    fn component(&mut self, component: &ComponentOp<'_>) -> Term<'a> {
        let mut props = self.props(&component.attributes, &component.bindings);
        let mut events = Vec::new();
        let mut own_slot = None;

        for binding in component.bindings.iter() {
            match binding {
                BindingOp::On(on) => events.push(EncodedEvent {
                    name: on.name.and_then(static_name),
                    modifiers: on.modifiers.iter().map(ToString::to_string).collect(),
                    value: on.handler.map(|handler| self.expression(handler)),
                }),
                // `v-model` on a component is a prop and its update event.
                BindingOp::Model(model) => {
                    let name = model
                        .argument
                        .and_then(static_name)
                        .unwrap_or_else(|| "modelValue".to_string());
                    props.push(EncodedProp {
                        name: Some(name.clone()),
                        name_value: None,
                        r#static: None,
                        value: Some(self.expression(model.contract.read)),
                    });
                    events.push(EncodedEvent {
                        name: Some(format!("update:{name}")),
                        modifiers: Vec::new(),
                        value: Some(self.expression(model.contract.write)),
                    });
                }
                BindingOp::SlotContent(slot) => own_slot = Some(slot),
                BindingOp::Bind(_) => {}
                other => {
                    let span = binding_span(other);
                    self.warning(
                        span.0,
                        span.1,
                        "this directive isn't applied to a component on the server",
                    );
                }
            }
        }

        let slots = match own_slot {
            // `<List v-slot="{ item }">`: the children are the default slot.
            Some(slot) => vec![EncodedSlotContent {
                name: slot
                    .name
                    .and_then(static_name)
                    .or_else(|| Some("default".to_string())),
                name_value: None,
                params: slot.params.map(|params| self.expression(params)),
                block: self.region(&component.children),
            }],
            None => self.slot_contents(&component.children),
        };

        EncodedComponentSlot {
            kind: atoms::component(),
            name: component.name.to_string(),
            props,
            events,
            slots,
            position: self.position(component.span.start),
        }
        .encode(self.env)
    }

    /// A component's children: `<template #name>` elements are named slots, and
    /// everything else is the default slot.
    fn slot_contents(&mut self, children: &Region<'_>) -> Vec<EncodedSlotContent<'a>> {
        let mut named = Vec::new();
        let mut default = Block::default();
        let mut has_default = false;

        for op in children.ops.iter() {
            match op {
                Op::Element(element) if element.tag == "template" => {
                    match element.bindings.iter().find_map(|binding| match binding {
                        BindingOp::SlotContent(slot) => Some(slot),
                        _ => None,
                    }) {
                        Some(slot) => {
                            let (name, name_value) = match slot.name {
                                Some(DynamicName::Static(name)) => (Some(name.to_string()), None),
                                Some(DynamicName::Dynamic(name)) => {
                                    (None, Some(self.expression(name)))
                                }
                                None => (Some("default".to_string()), None),
                            };
                            named.push(EncodedSlotContent {
                                name,
                                name_value,
                                params: slot.params.map(|params| self.expression(params)),
                                block: self.region(&element.children),
                            });
                        }
                        None => {
                            has_default = true;
                            self.op(&mut default, op);
                        }
                    }
                }
                Op::If(if_op)
                    if if_op
                        .branches
                        .iter()
                        .any(|branch| has_slot_template(&branch.region)) =>
                {
                    self.error(
                        if_op.span.start,
                        if_op.span.end,
                        "conditional slots aren't supported on the server yet",
                    );
                }
                Op::For(for_op) if has_slot_template(&for_op.region) => {
                    self.error(
                        for_op.span.start,
                        for_op.span.end,
                        "slots from v-for aren't supported on the server yet",
                    );
                }
                Op::Text(text) if text.content.trim().is_empty() => {
                    default.push(&escape_text(text.content))
                }
                Op::Comment(_) => {}
                _ => {
                    has_default = true;
                    self.op(&mut default, op);
                }
            }
        }

        if has_default {
            named.push(EncodedSlotContent {
                name: Some("default".to_string()),
                name_value: None,
                params: None,
                block: default.finish(),
            });
        }

        named
    }

    fn slot_outlet(&mut self, outlet: &SlotOp<'_>) -> Term<'a> {
        let (name, name_value) = match outlet.name {
            DynamicName::Static(name) => (Some(name.to_string()), None),
            DynamicName::Dynamic(name) => (None, Some(self.expression(name))),
        };

        let fallback = if outlet.fallback.ops.is_empty() {
            None
        } else {
            Some(self.region(&outlet.fallback))
        };

        EncodedSlotOutlet {
            kind: atoms::slot(),
            name,
            name_value,
            props: self.props(&outlet.attributes, &outlet.bindings),
            fallback,
            position: self.position(outlet.span.start),
        }
        .encode(self.env)
    }

    fn if_slot(&mut self, if_op: &IfOp<'_>) -> Term<'a> {
        let branches = if_op
            .branches
            .iter()
            .map(|branch| EncodedIfBranch {
                condition: branch.condition.map(|condition| self.expression(condition)),
                block: self.region(&branch.region),
            })
            .collect();

        EncodedIfSlot {
            kind: atoms::r#if(),
            branches,
            position: self.position(if_op.span.start),
        }
        .encode(self.env)
    }

    fn for_slot(&mut self, for_op: &ForOp<'_>) -> Term<'a> {
        let binding = for_op.binding;
        let key_prop = loop_key(&for_op.region).map(|key| self.expression(key));

        EncodedForSlot {
            kind: atoms::r#for(),
            source: self.expression(binding.source),
            value: self.expression(binding.value),
            key: binding.key.map(|key| self.expression(key)),
            index: binding.index.map(|index| self.expression(index)),
            key_prop,
            block: self.region(&for_op.region),
            position: self.position(for_op.span.start),
        }
        .encode(self.env)
    }

    /// The expression's source. One Vize couldn't parse is reported, and its
    /// source is returned anyway so splitting can go on to find more problems.
    fn expression(&mut self, expression: ExprRef<'_>) -> String {
        if let ExprRef::Opaque(opaque) = expression {
            let message = format!("can't parse the expression `{}`", opaque.source.trim());
            self.error(opaque.span.start, opaque.span.end, &message);
        }
        expression.source().trim().to_string()
    }

    fn position(&self, offset: u32) -> Position {
        let (line, column) = self.lines.line_col(offset as usize);
        (line as usize + 1, column as usize + 1)
    }

    fn encoded_position(&self, offset: u32) -> EncodedPosition {
        let (line, column) = self.position(offset);
        EncodedPosition {
            offset,
            line: line as u32,
            column: column as u32,
        }
    }

    fn error(&mut self, start: u32, end: u32, message: &str) {
        self.diagnose(atoms::error(), start, end, message);
    }

    fn warning(&mut self, start: u32, end: u32, message: &str) {
        self.diagnose(atoms::warning(), start, end, message);
    }

    fn diagnose(&mut self, severity: rustler::Atom, start: u32, end: u32, message: &str) {
        let diagnostic = EncodedSplitDiagnostic {
            severity,
            message: message.to_string(),
            start: self.encoded_position(start),
            end: self.encoded_position(end),
        };
        self.diagnostics.push(diagnostic);
    }
}

fn lowering_diagnostic(diagnostic: &Diagnostic, source: &str) -> Option<EncodedSplitDiagnostic> {
    let severity = match diagnostic.severity() {
        Severity::Error => atoms::error(),
        Severity::Warning => atoms::warning(),
        Severity::Info | Severity::Hint => return None,
    };

    let lines = LineIndex::new(source);
    let position = |offset: u32| {
        let (line, column) = lines.line_col(offset as usize);
        EncodedPosition {
            offset,
            line: line + 1,
            column: column + 1,
        }
    };

    Some(EncodedSplitDiagnostic {
        severity,
        message: diagnostic.message.to_string(),
        start: position(diagnostic.span.start),
        end: position(diagnostic.span.end),
    })
}

/// The names bindings set statically, such as `class` for `:class`.
fn bound_names<'b>(bindings: &'b [BindingOp<'_>]) -> Vec<&'b str> {
    bindings
        .iter()
        .filter_map(|binding| match binding {
            BindingOp::Bind(bind) => match bind.name {
                Some(DynamicName::Static(name)) => Some(name),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn static_name(name: DynamicName<'_>) -> Option<String> {
    match name {
        DynamicName::Static(name) => Some(name.to_string()),
        DynamicName::Dynamic(_) => None,
    }
}

fn static_value(attributes: &[Attribute<'_>], name: &str) -> Option<String> {
    attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .map(|attribute| decode_html_attribute_entities(attribute.value.unwrap_or("")).to_string())
}

fn static_prop(attribute: &Attribute<'_>) -> EncodedProp {
    EncodedProp {
        name: Some(attribute.name.to_string()),
        name_value: None,
        r#static: Some(decode_html_attribute_entities(attribute.value.unwrap_or("")).to_string()),
        value: None,
    }
}

fn static_attribute(attribute: &Attribute<'_>) -> String {
    match attribute.value {
        Some(value) => {
            let value = decode_html_attribute_entities(value);
            format!(
                " {}=\"{}\"",
                attribute.name,
                html_escape::encode_double_quoted_attribute(&value)
            )
        }
        None => format!(" {}", attribute.name),
    }
}

fn escape_text(raw: &str) -> String {
    html_escape::encode_text(&decode_ssr_static_text(raw)).into_owned()
}

/// The `:key` of a `v-for`'s repeated element.
fn loop_key<'r>(region: &'r Region<'r>) -> Option<ExprRef<'r>> {
    let bindings = match region.ops.as_slice() {
        [Op::Element(element)] => &element.bindings,
        [Op::Component(component)] => &component.bindings,
        _ => return None,
    };

    bindings.iter().find_map(|binding| match binding {
        BindingOp::Bind(bind) if matches!(bind.name, Some(DynamicName::Static("key"))) => {
            bind.value
        }
        _ => None,
    })
}

fn has_slot_template(region: &Region<'_>) -> bool {
    region.ops.iter().any(|op| match op {
        Op::Element(element) => element
            .bindings
            .iter()
            .any(|binding| matches!(binding, BindingOp::SlotContent(_))),
        _ => false,
    })
}

fn binding_span(binding: &BindingOp<'_>) -> (u32, u32) {
    let span = match binding {
        BindingOp::Bind(op) => op.span,
        BindingOp::On(op) => op.span,
        BindingOp::Model(op) => op.span,
        BindingOp::SlotContent(op) => op.span,
        BindingOp::VueDirective(op) => op.span,
        BindingOp::VueCssBind(op) => op.span,
        BindingOp::VueSync(op) => op.span,
        BindingOp::VueSlotScope(op) => op.span,
        BindingOp::VueOnce(op) => op.span,
        BindingOp::VueMemo(op) => op.span,
        BindingOp::VueShow(op) => op.span,
        BindingOp::VueHtml(op) => op.span,
        BindingOp::VueText(op) => op.span,
        BindingOp::VueCloak(op) => op.span,
    };
    (span.start, span.end)
}
