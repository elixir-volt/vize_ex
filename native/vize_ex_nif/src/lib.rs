use rustler::{Encoder, Env, NifResult, Term};
use vize_atelier_core::options::{CodegenMode, CodegenOptions, ParserOptions, TransformOptions};
use vize_atelier_core::parser::{parse, parse_with_options};
use vize_atelier_core::transform::transform;
use vize_atelier_sfc::{compile_sfc, parse_sfc, SfcCompileOptions, SfcParseOptions};
use vize_atelier_ssr::compile_ssr;
use vize_atelier_vapor::{compile_vapor, ir::*, transform_to_ir, VaporCompilerOptions};
use vize_carton::Bump;

mod atoms {
    rustler::atoms! {
        ok,
        error,

        // SFC descriptor fields
        template,
        script,
        script_setup,
        styles,
        custom_blocks,
        content,
        lang,
        scoped,
        module,
        setup,
        attrs,
        block_type,
        loc,
        start,
        end_,    // `end` is a Rust keyword
        start_line,
        start_column,
        end_line,
        end_column,

        // Compile result fields
        code,
        css,
        errors,
        warnings,
        message,
        preamble,
        helpers,
        templates,

        // Vapor IR fields
        root,
        block,
        operations,
        effects,
        returns,
        element,
        key,
        values,
        value,
        condition,
        positive,
        negative,
        source,
        index,
        render,
        once,
        parent,
        anchor,
        tag,
        props,
        slots,
        name,
        fallback,
        delegate,
        effect,
        modifiers,
        camel,
        prop_modifier,
        is_component,
        components,
        directives,
        kind,
        asset,
        dynamic_slots,
        child_id,
        parent_id,
        offset,

        // IR node type atoms
        set_prop,
        set_dynamic_props,
        set_text,
        set_event,
        set_html,
        set_template_ref,
        insert_node,
        prepend_node,
        directive,
        if_node,
        for_node,
        create_component,
        slot_outlet,
        get_text_child,
        child_ref,
        next_ref,

        // Component kinds
        regular,
        teleport,
        keep_alive,
        suspense,
        dynamic,
    }
}

fn loc_to_term<'a>(env: Env<'a>, loc: &vize_atelier_sfc::BlockLocation) -> Term<'a> {
    Term::map_from_arrays(
        env,
        &[
            atoms::start().encode(env),
            atoms::end_().encode(env),
            atoms::start_line().encode(env),
            atoms::start_column().encode(env),
            atoms::end_line().encode(env),
            atoms::end_column().encode(env),
        ],
        &[
            loc.start.encode(env),
            loc.end.encode(env),
            loc.start_line.encode(env),
            loc.start_column.encode(env),
            loc.end_line.encode(env),
            loc.end_column.encode(env),
        ],
    )
    .unwrap()
}

fn attrs_to_term<'a>(
    env: Env<'a>,
    attrs: &vize_carton::FxHashMap<std::borrow::Cow<'_, str>, std::borrow::Cow<'_, str>>,
) -> Term<'a> {
    let keys: Vec<Term<'a>> = attrs.keys().map(|k| k.as_ref().encode(env)).collect();
    let vals: Vec<Term<'a>> = attrs.values().map(|v| v.as_ref().encode(env)).collect();
    if keys.is_empty() {
        Term::map_new(env)
    } else {
        Term::map_from_arrays(env, &keys, &vals).unwrap()
    }
}

fn sfc_error_to_term<'a>(env: Env<'a>, err: &vize_atelier_sfc::SfcError) -> Term<'a> {
    let mut keys = vec![atoms::message().encode(env)];
    let mut vals: Vec<Term<'a>> = vec![err.message.as_str().encode(env)];
    if let Some(ref code) = err.code {
        keys.push(atoms::code().encode(env));
        vals.push(code.as_str().encode(env));
    }
    Term::map_from_arrays(env, &keys, &vals).unwrap()
}

// ── SFC Parsing ──

#[rustler::nif(schedule = "DirtyCpu")]
fn parse_sfc_nif<'a>(env: Env<'a>, source: &str) -> NifResult<Term<'a>> {
    let opts = SfcParseOptions::default();
    match parse_sfc(source, opts) {
        Ok(descriptor) => {
            let template_term = match &descriptor.template {
                Some(t) => {
                    let map = Term::map_from_arrays(
                        env,
                        &[
                            atoms::content().encode(env),
                            atoms::lang().encode(env),
                            atoms::loc().encode(env),
                            atoms::attrs().encode(env),
                        ],
                        &[
                            t.content.as_ref().encode(env),
                            t.lang.as_deref().encode(env),
                            loc_to_term(env, &t.loc),
                            attrs_to_term(env, &t.attrs),
                        ],
                    )
                    .unwrap();
                    map.encode(env)
                }
                None => rustler::types::atom::nil().encode(env),
            };

            let script_term = encode_script_block(env, descriptor.script.as_ref());
            let script_setup_term = encode_script_block(env, descriptor.script_setup.as_ref());

            let styles_term: Vec<Term<'a>> = descriptor
                .styles
                .iter()
                .map(|s| {
                    Term::map_from_arrays(
                        env,
                        &[
                            atoms::content().encode(env),
                            atoms::lang().encode(env),
                            atoms::scoped().encode(env),
                            atoms::module().encode(env),
                            atoms::loc().encode(env),
                            atoms::attrs().encode(env),
                        ],
                        &[
                            s.content.as_ref().encode(env),
                            s.lang.as_deref().encode(env),
                            s.scoped.encode(env),
                            s.module.as_deref().encode(env),
                            loc_to_term(env, &s.loc),
                            attrs_to_term(env, &s.attrs),
                        ],
                    )
                    .unwrap()
                })
                .collect();

            let custom_blocks_term: Vec<Term<'a>> = descriptor
                .custom_blocks
                .iter()
                .map(|c| {
                    Term::map_from_arrays(
                        env,
                        &[
                            atoms::block_type().encode(env),
                            atoms::content().encode(env),
                            atoms::loc().encode(env),
                            atoms::attrs().encode(env),
                        ],
                        &[
                            c.block_type.as_ref().encode(env),
                            c.content.as_ref().encode(env),
                            loc_to_term(env, &c.loc),
                            attrs_to_term(env, &c.attrs),
                        ],
                    )
                    .unwrap()
                })
                .collect();

            let result = Term::map_from_arrays(
                env,
                &[
                    atoms::template().encode(env),
                    atoms::script().encode(env),
                    atoms::script_setup().encode(env),
                    atoms::styles().encode(env),
                    atoms::custom_blocks().encode(env),
                ],
                &[
                    template_term,
                    script_term,
                    script_setup_term,
                    styles_term.encode(env),
                    custom_blocks_term.encode(env),
                ],
            )
            .unwrap();

            Ok((atoms::ok(), result).encode(env))
        }
        Err(e) => {
            let msg = format!("{e:?}");
            Ok((atoms::error(), msg).encode(env))
        }
    }
}

fn encode_script_block<'a>(
    env: Env<'a>,
    block: Option<&vize_atelier_sfc::SfcScriptBlock>,
) -> Term<'a> {
    match block {
        Some(s) => Term::map_from_arrays(
            env,
            &[
                atoms::content().encode(env),
                atoms::lang().encode(env),
                atoms::setup().encode(env),
                atoms::loc().encode(env),
                atoms::attrs().encode(env),
            ],
            &[
                s.content.as_ref().encode(env),
                s.lang.as_deref().encode(env),
                s.setup.encode(env),
                loc_to_term(env, &s.loc),
                attrs_to_term(env, &s.attrs),
            ],
        )
        .unwrap(),
        None => rustler::types::atom::nil().encode(env),
    }
}

// ── SFC Compilation ──

#[rustler::nif(schedule = "DirtyCpu")]
fn compile_sfc_nif<'a>(env: Env<'a>, source: &str, vapor: bool, ssr: bool) -> NifResult<Term<'a>> {
    let opts = SfcParseOptions::default();
    let descriptor = match parse_sfc(source, opts) {
        Ok(d) => d,
        Err(e) => {
            let msg = format!("{e:?}");
            return Ok((atoms::error(), msg).encode(env));
        }
    };

    let compile_opts = SfcCompileOptions {
        vapor,
        template: vize_atelier_sfc::TemplateCompileOptions {
            ssr,
            ..Default::default()
        },
        ..Default::default()
    };

    match compile_sfc(&descriptor, compile_opts) {
        Ok(result) => {
            let errors_term: Vec<Term<'a>> = result
                .errors
                .iter()
                .map(|e| sfc_error_to_term(env, e))
                .collect();
            let warnings_term: Vec<Term<'a>> = result
                .warnings
                .iter()
                .map(|e| sfc_error_to_term(env, e))
                .collect();

            let map = Term::map_from_arrays(
                env,
                &[
                    atoms::code().encode(env),
                    atoms::css().encode(env),
                    atoms::errors().encode(env),
                    atoms::warnings().encode(env),
                ],
                &[
                    result.code.as_str().encode(env),
                    result.css.as_deref().encode(env),
                    errors_term.encode(env),
                    warnings_term.encode(env),
                ],
            )
            .unwrap();

            Ok((atoms::ok(), map).encode(env))
        }
        Err(e) => {
            let msg = e.message.as_str();
            Ok((atoms::error(), msg).encode(env))
        }
    }
}

// ── Template Compilation ──

#[rustler::nif(schedule = "DirtyCpu")]
fn compile_template_nif<'a>(
    env: Env<'a>,
    source: &str,
    mode: &str,
    ssr: bool,
) -> NifResult<Term<'a>> {
    let allocator = Bump::new();
    let (mut root, errors) = parse(&allocator, source);

    if !errors.is_empty() {
        let msgs: Vec<std::string::String> = errors.iter().map(|e| e.message.to_string()).collect();
        return Ok((atoms::error(), msgs).encode(env));
    }

    let is_module = mode == "module";
    let transform_opts = TransformOptions {
        prefix_identifiers: is_module,
        ssr,
        ..Default::default()
    };
    transform(&allocator, &mut root, transform_opts, None);

    let codegen_opts = CodegenOptions {
        mode: if is_module {
            CodegenMode::Module
        } else {
            CodegenMode::Function
        },
        ssr,
        ..Default::default()
    };
    let result = vize_atelier_core::codegen::generate(&root, codegen_opts);

    let helpers: Vec<&str> = root.helpers.iter().map(|h| h.name()).collect();

    let map = Term::map_from_arrays(
        env,
        &[
            atoms::code().encode(env),
            atoms::preamble().encode(env),
            atoms::helpers().encode(env),
        ],
        &[
            result.code.as_str().encode(env),
            result.preamble.as_str().encode(env),
            helpers.encode(env),
        ],
    )
    .unwrap();

    Ok((atoms::ok(), map).encode(env))
}

// ── SSR Compilation ──

#[rustler::nif(schedule = "DirtyCpu")]
fn compile_ssr_nif<'a>(env: Env<'a>, source: &str) -> NifResult<Term<'a>> {
    let allocator = Bump::new();
    let (_root, errors, result) = compile_ssr(&allocator, source);

    if !errors.is_empty() {
        let msgs: Vec<std::string::String> = errors.iter().map(|e| e.message.to_string()).collect();
        return Ok((atoms::error(), msgs).encode(env));
    }

    let map = Term::map_from_arrays(
        env,
        &[atoms::code().encode(env), atoms::preamble().encode(env)],
        &[
            result.code.as_str().encode(env),
            result.preamble.as_str().encode(env),
        ],
    )
    .unwrap();

    Ok((atoms::ok(), map).encode(env))
}

// ── Vapor Compilation ──

#[rustler::nif(schedule = "DirtyCpu")]
fn compile_vapor_nif<'a>(env: Env<'a>, source: &str, ssr: bool) -> NifResult<Term<'a>> {
    let allocator = Bump::new();
    let opts = VaporCompilerOptions {
        ssr,
        ..Default::default()
    };
    let result = compile_vapor(&allocator, source, opts);

    if !result.error_messages.is_empty() {
        let msgs: Vec<&str> = result.error_messages.iter().map(|s| s.as_str()).collect();
        return Ok((atoms::error(), msgs).encode(env));
    }

    let templates: Vec<&str> = result.templates.iter().map(|s| s.as_str()).collect();

    let map = Term::map_from_arrays(
        env,
        &[atoms::code().encode(env), atoms::templates().encode(env)],
        &[result.code.as_str().encode(env), templates.encode(env)],
    )
    .unwrap();

    Ok((atoms::ok(), map).encode(env))
}

// ── Vapor IR ──

fn encode_simple_expr<'a>(
    env: Env<'a>,
    expr: &vize_atelier_core::SimpleExpressionNode,
) -> Term<'a> {
    expr.content.as_str().encode(env)
}

fn encode_ir_prop<'a>(env: Env<'a>, prop: &IRProp) -> Term<'a> {
    let values: Vec<Term<'a>> = prop
        .values
        .iter()
        .map(|v| encode_simple_expr(env, v))
        .collect();

    Term::map_from_arrays(
        env,
        &[
            atoms::key().encode(env),
            atoms::values().encode(env),
            atoms::is_component().encode(env),
        ],
        &[
            encode_simple_expr(env, &prop.key),
            values.encode(env),
            prop.is_component.encode(env),
        ],
    )
    .unwrap()
}

fn encode_operation<'a>(env: Env<'a>, op: &OperationNode) -> Term<'a> {
    match op {
        OperationNode::SetProp(node) => {
            let prop = encode_ir_prop(env, &node.prop);
            Term::map_from_arrays(
                env,
                &[
                    atoms::kind().encode(env),
                    atoms::element().encode(env),
                    atoms::tag().encode(env),
                    atoms::camel().encode(env),
                    atoms::prop_modifier().encode(env),
                    atoms::value().encode(env),
                ],
                &[
                    atoms::set_prop().encode(env),
                    node.element.encode(env),
                    node.tag.as_str().encode(env),
                    node.camel.encode(env),
                    node.prop_modifier.encode(env),
                    prop,
                ],
            )
            .unwrap()
        }
        OperationNode::SetDynamicProps(node) => {
            let props: Vec<Term<'a>> = node
                .props
                .iter()
                .map(|p| encode_simple_expr(env, p))
                .collect();
            Term::map_from_arrays(
                env,
                &[
                    atoms::kind().encode(env),
                    atoms::element().encode(env),
                    atoms::props().encode(env),
                ],
                &[
                    atoms::set_dynamic_props().encode(env),
                    node.element.encode(env),
                    props.encode(env),
                ],
            )
            .unwrap()
        }
        OperationNode::SetText(node) => {
            let values: Vec<Term<'a>> = node
                .values
                .iter()
                .map(|v| encode_simple_expr(env, v))
                .collect();
            Term::map_from_arrays(
                env,
                &[
                    atoms::kind().encode(env),
                    atoms::element().encode(env),
                    atoms::values().encode(env),
                ],
                &[
                    atoms::set_text().encode(env),
                    node.element.encode(env),
                    values.encode(env),
                ],
            )
            .unwrap()
        }
        OperationNode::SetEvent(node) => Term::map_from_arrays(
            env,
            &[
                atoms::kind().encode(env),
                atoms::element().encode(env),
                atoms::key().encode(env),
                atoms::value().encode(env),
                atoms::delegate().encode(env),
                atoms::effect().encode(env),
            ],
            &[
                atoms::set_event().encode(env),
                node.element.encode(env),
                encode_simple_expr(env, &node.key),
                node.value
                    .as_ref()
                    .map(|v| encode_simple_expr(env, v))
                    .unwrap_or_else(|| rustler::types::atom::nil().encode(env)),
                node.delegate.encode(env),
                node.effect.encode(env),
            ],
        )
        .unwrap(),
        OperationNode::SetHtml(node) => Term::map_from_arrays(
            env,
            &[
                atoms::kind().encode(env),
                atoms::element().encode(env),
                atoms::value().encode(env),
            ],
            &[
                atoms::set_html().encode(env),
                node.element.encode(env),
                encode_simple_expr(env, &node.value),
            ],
        )
        .unwrap(),
        OperationNode::SetTemplateRef(node) => Term::map_from_arrays(
            env,
            &[
                atoms::kind().encode(env),
                atoms::element().encode(env),
                atoms::value().encode(env),
            ],
            &[
                atoms::set_template_ref().encode(env),
                node.element.encode(env),
                encode_simple_expr(env, &node.value),
            ],
        )
        .unwrap(),
        OperationNode::InsertNode(node) => {
            let elements: Vec<usize> = node.elements.clone();
            Term::map_from_arrays(
                env,
                &[
                    atoms::kind().encode(env),
                    atoms::element().encode(env),
                    atoms::parent().encode(env),
                    atoms::anchor().encode(env),
                ],
                &[
                    atoms::insert_node().encode(env),
                    elements.encode(env),
                    node.parent.encode(env),
                    node.anchor.encode(env),
                ],
            )
            .unwrap()
        }
        OperationNode::PrependNode(node) => {
            let elements: Vec<usize> = node.elements.clone();
            Term::map_from_arrays(
                env,
                &[
                    atoms::kind().encode(env),
                    atoms::element().encode(env),
                    atoms::parent().encode(env),
                ],
                &[
                    atoms::prepend_node().encode(env),
                    elements.encode(env),
                    node.parent.encode(env),
                ],
            )
            .unwrap()
        }
        OperationNode::If(if_node) => encode_if_node(env, if_node),
        OperationNode::For(for_node) => encode_for_node(env, for_node),
        OperationNode::CreateComponent(node) => {
            let props: Vec<Term<'a>> = node.props.iter().map(|p| encode_ir_prop(env, p)).collect();
            let kind_atom = match node.kind {
                ComponentKind::Regular => atoms::regular(),
                ComponentKind::Teleport => atoms::teleport(),
                ComponentKind::KeepAlive => atoms::keep_alive(),
                ComponentKind::Suspense => atoms::suspense(),
                ComponentKind::Dynamic => atoms::dynamic(),
            };
            Term::map_from_arrays(
                env,
                &[
                    atoms::kind().encode(env),
                    atoms::tag().encode(env),
                    atoms::props().encode(env),
                    atoms::asset().encode(env),
                    atoms::once().encode(env),
                    atoms::dynamic_slots().encode(env),
                    atoms::parent().encode(env),
                    atoms::anchor().encode(env),
                    atoms::value().encode(env),
                ],
                &[
                    atoms::create_component().encode(env),
                    node.tag.as_str().encode(env),
                    props.encode(env),
                    node.asset.encode(env),
                    node.once.encode(env),
                    node.dynamic_slots.encode(env),
                    node.parent.encode(env),
                    node.anchor.encode(env),
                    kind_atom.encode(env),
                ],
            )
            .unwrap()
        }
        OperationNode::SlotOutlet(node) => Term::map_from_arrays(
            env,
            &[
                atoms::kind().encode(env),
                atoms::name().encode(env),
                atoms::props().encode(env),
            ],
            &[
                atoms::slot_outlet().encode(env),
                encode_simple_expr(env, &node.name),
                node.props
                    .iter()
                    .map(|p| encode_ir_prop(env, p))
                    .collect::<Vec<_>>()
                    .encode(env),
            ],
        )
        .unwrap(),
        OperationNode::Directive(node) => Term::map_from_arrays(
            env,
            &[
                atoms::kind().encode(env),
                atoms::element().encode(env),
                atoms::name().encode(env),
                atoms::tag().encode(env),
            ],
            &[
                atoms::directive().encode(env),
                node.element.encode(env),
                node.name.as_str().encode(env),
                node.tag.as_str().encode(env),
            ],
        )
        .unwrap(),
        OperationNode::GetTextChild(node) => Term::map_from_arrays(
            env,
            &[atoms::kind().encode(env), atoms::parent().encode(env)],
            &[atoms::get_text_child().encode(env), node.parent.encode(env)],
        )
        .unwrap(),
        OperationNode::ChildRef(node) => Term::map_from_arrays(
            env,
            &[
                atoms::kind().encode(env),
                atoms::child_id().encode(env),
                atoms::parent_id().encode(env),
                atoms::offset().encode(env),
            ],
            &[
                atoms::child_ref().encode(env),
                node.child_id.encode(env),
                node.parent_id.encode(env),
                node.offset.encode(env),
            ],
        )
        .unwrap(),
        OperationNode::NextRef(node) => Term::map_from_arrays(
            env,
            &[
                atoms::kind().encode(env),
                atoms::child_id().encode(env),
                atoms::parent_id().encode(env),
                atoms::offset().encode(env),
            ],
            &[
                atoms::next_ref().encode(env),
                node.child_id.encode(env),
                node.prev_id.encode(env),
                node.offset.encode(env),
            ],
        )
        .unwrap(),
    }
}

fn encode_block<'a>(env: Env<'a>, block: &BlockIRNode) -> Term<'a> {
    let operations: Vec<Term<'a>> = block
        .operation
        .iter()
        .map(|op| encode_operation(env, op))
        .collect();

    let effects: Vec<Term<'a>> = block
        .effect
        .iter()
        .map(|eff| {
            let ops: Vec<Term<'a>> = eff
                .operations
                .iter()
                .map(|op| encode_operation(env, op))
                .collect();
            ops.encode(env)
        })
        .collect();

    let returns: Vec<usize> = block.returns.iter().copied().collect();

    Term::map_from_arrays(
        env,
        &[
            atoms::operations().encode(env),
            atoms::effects().encode(env),
            atoms::returns().encode(env),
        ],
        &[
            operations.encode(env),
            effects.encode(env),
            returns.encode(env),
        ],
    )
    .unwrap()
}

fn encode_if_node<'a>(env: Env<'a>, if_node: &IfIRNode) -> Term<'a> {
    let negative_term = match &if_node.negative {
        Some(NegativeBranch::Block(block)) => encode_block(env, block),
        Some(NegativeBranch::If(nested)) => encode_if_node(env, nested),
        None => rustler::types::atom::nil().encode(env),
    };

    Term::map_from_arrays(
        env,
        &[
            atoms::kind().encode(env),
            atoms::condition().encode(env),
            atoms::positive().encode(env),
            atoms::negative().encode(env),
            atoms::once().encode(env),
            atoms::parent().encode(env),
            atoms::anchor().encode(env),
        ],
        &[
            atoms::if_node().encode(env),
            encode_simple_expr(env, &if_node.condition),
            encode_block(env, &if_node.positive),
            negative_term,
            if_node.once.encode(env),
            if_node.parent.encode(env),
            if_node.anchor.encode(env),
        ],
    )
    .unwrap()
}

fn encode_for_node<'a>(env: Env<'a>, for_node: &ForIRNode) -> Term<'a> {
    let nil = rustler::types::atom::nil().encode(env);

    Term::map_from_arrays(
        env,
        &[
            atoms::kind().encode(env),
            atoms::source().encode(env),
            atoms::value().encode(env),
            atoms::key().encode(env),
            atoms::index().encode(env),
            atoms::render().encode(env),
            atoms::once().encode(env),
            atoms::parent().encode(env),
            atoms::anchor().encode(env),
        ],
        &[
            atoms::for_node().encode(env),
            encode_simple_expr(env, &for_node.source),
            for_node
                .value
                .as_ref()
                .map(|v| encode_simple_expr(env, v))
                .unwrap_or(nil),
            for_node
                .key
                .as_ref()
                .map(|v| encode_simple_expr(env, v))
                .unwrap_or(nil),
            for_node
                .index
                .as_ref()
                .map(|v| encode_simple_expr(env, v))
                .unwrap_or(nil),
            encode_block(env, &for_node.render),
            for_node.once.encode(env),
            for_node.parent.encode(env),
            for_node.anchor.encode(env),
        ],
    )
    .unwrap()
}

#[rustler::nif(schedule = "DirtyCpu")]
fn vapor_ir_nif<'a>(env: Env<'a>, source: &str) -> NifResult<Term<'a>> {
    let allocator = Bump::new();
    let parser_opts = ParserOptions::default();
    let (mut root, errors) = parse_with_options(&allocator, source, parser_opts);

    if !errors.is_empty() {
        let msgs: Vec<std::string::String> = errors.iter().map(|e| e.message.to_string()).collect();
        return Ok((atoms::error(), msgs).encode(env));
    }

    let transform_opts = TransformOptions {
        vapor: true,
        ..Default::default()
    };
    transform(&allocator, &mut root, transform_opts, None);

    let ir = transform_to_ir(&allocator, &root);

    let templates: Vec<&str> = ir.templates.iter().map(|s| s.as_str()).collect();
    let components: Vec<&str> = ir.component.iter().map(|s| s.as_str()).collect();
    let directives: Vec<&str> = ir.directive.iter().map(|s| s.as_str()).collect();

    let map = Term::map_from_arrays(
        env,
        &[
            atoms::templates().encode(env),
            atoms::components().encode(env),
            atoms::directives().encode(env),
            atoms::block().encode(env),
        ],
        &[
            templates.encode(env),
            components.encode(env),
            directives.encode(env),
            encode_block(env, &ir.block),
        ],
    )
    .unwrap();

    Ok((atoms::ok(), map).encode(env))
}

// ── Linting ──

#[rustler::nif(schedule = "DirtyCpu")]
fn lint_nif<'a>(env: Env<'a>, source: &str, filename: &str) -> NifResult<Term<'a>> {
    use vize_patina::Linter;

    let linter = Linter::default();
    let result = linter.lint_sfc(source, filename);
    let diagnostics: Vec<Term<'a>> = result
        .diagnostics
        .iter()
        .map(|d| {
            let keys = vec![atoms::message().encode(env), atoms::name().encode(env)];
            let vals: Vec<Term<'a>> = vec![d.message.as_str().encode(env), d.rule_name.encode(env)];

            Term::map_from_arrays(env, &keys, &vals).unwrap()
        })
        .collect();

    Ok((atoms::ok(), diagnostics).encode(env))
}

rustler::init!("Elixir.Vize.Native");
