use rustler::{Encoder, Env, NifResult, Term};
use vize_atelier_core::options::{CodegenMode, CodegenOptions, ParserOptions, TransformOptions};
use vize_atelier_core::parser::{parse, parse_with_options};
use vize_atelier_core::transform::transform;
use vize_atelier_sfc::{
    bundle_css, compile_css, compile_sfc, parse_sfc, CssCompileOptions, CssTargets, SfcCompileOptions,
    SfcParseOptions,
};
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
        template_hash,
        style_hash,
        script_hash,
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
        key_prop,
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

        // CSS result fields
        map,
        css_vars,
        exports,
        minify,
        targets,
        scope_id,
        filename_opt,
        custom_media,

        // Expression tags
        static_,
        element_template_map,

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

        // Split result fields
        statics,

        // Directive kinds
        v_show,
        v_model,

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
fn compile_sfc_nif<'a>(
    env: Env<'a>,
    source: &str,
    filename: &str,
    scope_id: &str,
    vapor: bool,
    ssr: bool,
) -> NifResult<Term<'a>> {
    let mut parse_opts = SfcParseOptions::default();
    if !filename.is_empty() {
        parse_opts.filename = filename.into();
    }

    let descriptor = match parse_sfc(source, parse_opts) {
        Ok(d) => d,
        Err(e) => {
            let msg = format!("{e:?}");
            return Ok((atoms::error(), msg).encode(env));
        }
    };

    let mut compile_opts = SfcCompileOptions {
        vapor,
        template: vize_atelier_sfc::TemplateCompileOptions {
            ssr,
            ..Default::default()
        },
        ..Default::default()
    };
    if !scope_id.is_empty() {
        compile_opts.scope_id = Some(scope_id.into());
    }
    if !filename.is_empty() {
        compile_opts.script.id = Some(filename.into());
    }

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

            let template_hash = descriptor.template_hash();
            let style_hash = descriptor.style_hash();
            let script_hash = descriptor.script_hash();

            let map = Term::map_from_arrays(
                env,
                &[
                    atoms::code().encode(env),
                    atoms::css().encode(env),
                    atoms::errors().encode(env),
                    atoms::warnings().encode(env),
                    atoms::template_hash().encode(env),
                    atoms::style_hash().encode(env),
                    atoms::script_hash().encode(env),
                ],
                &[
                    result.code.as_str().encode(env),
                    result.css.as_deref().encode(env),
                    errors_term.encode(env),
                    warnings_term.encode(env),
                    template_hash.as_deref().encode(env),
                    style_hash.as_deref().encode(env),
                    script_hash.as_deref().encode(env),
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
    if expr.is_static {
        rustler::types::tuple::make_tuple(
            env,
            &[
                atoms::static_().encode(env),
                expr.content.as_str().encode(env),
            ],
        )
    } else {
        expr.content.as_str().encode(env)
    }
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
        OperationNode::Directive(node) => {
            let exp = node
                .dir
                .exp
                .as_ref()
                .map(|e| match e {
                    vize_atelier_core::ExpressionNode::Simple(s) => encode_simple_expr(env, s),
                    vize_atelier_core::ExpressionNode::Compound(c) => {
                        // For compound expressions, join children as a string
                        let content: std::string::String = c
                            .children
                            .iter()
                            .map(|child| match child {
                                vize_atelier_core::CompoundExpressionChild::Simple(s) => {
                                    s.content.to_string()
                                }
                                vize_atelier_core::CompoundExpressionChild::String(s) => {
                                    s.to_string()
                                }
                                _ => std::string::String::new(),
                            })
                            .collect();
                        content.as_str().encode(env)
                    }
                })
                .unwrap_or_else(|| rustler::types::atom::nil().encode(env));

            Term::map_from_arrays(
                env,
                &[
                    atoms::kind().encode(env),
                    atoms::element().encode(env),
                    atoms::name().encode(env),
                    atoms::tag().encode(env),
                    atoms::value().encode(env),
                ],
                &[
                    atoms::directive().encode(env),
                    node.element.encode(env),
                    node.name.as_str().encode(env),
                    node.tag.as_str().encode(env),
                    exp,
                ],
            )
            .unwrap()
        }
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
            atoms::key_prop().encode(env),
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
            for_node
                .key_prop
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

    // Element ID → template index mapping
    let etm_keys: Vec<usize> = ir.element_template_map.keys().copied().collect();
    let etm_vals: Vec<usize> = etm_keys
        .iter()
        .map(|k| ir.element_template_map[k])
        .collect();
    let element_template_map: Vec<(usize, usize)> =
        etm_keys.into_iter().zip(etm_vals.into_iter()).collect();

    let map = Term::map_from_arrays(
        env,
        &[
            atoms::templates().encode(env),
            atoms::components().encode(env),
            atoms::directives().encode(env),
            atoms::block().encode(env),
            atoms::element_template_map().encode(env),
        ],
        &[
            templates.encode(env),
            components.encode(env),
            directives.encode(env),
            encode_block(env, &ir.block),
            element_template_map.encode(env),
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

// ── CSS Compilation ──

#[rustler::nif(schedule = "DirtyCpu")]
#[allow(clippy::too_many_arguments)]
fn compile_css_nif<'a>(
    env: Env<'a>,
    source: &str,
    minify: bool,
    scoped: bool,
    scope_id_str: &str,
    filename: &str,
    chrome: i64,
    firefox: i64,
    safari: i64,
    css_modules: bool,
) -> NifResult<Term<'a>> {
    let targets = if chrome >= 0 || firefox >= 0 || safari >= 0 {
        Some(CssTargets {
            chrome: if chrome >= 0 {
                Some(chrome as u32)
            } else {
                None
            },
            firefox: if firefox >= 0 {
                Some(firefox as u32)
            } else {
                None
            },
            safari: if safari >= 0 {
                Some(safari as u32)
            } else {
                None
            },
            ..Default::default()
        })
    } else {
        None
    };

    let options = CssCompileOptions {
        scope_id: if scope_id_str.is_empty() {
            None
        } else {
            Some(scope_id_str.into())
        },
        scoped,
        minify,
        source_map: false,
        targets,
        filename: if filename.is_empty() {
            None
        } else {
            Some(filename.into())
        },
        custom_media: false,
        css_modules,
    };

    let result = compile_css(source, &options);

    let css_vars: Vec<&str> = result.css_vars.iter().map(|s| s.as_str()).collect();
    let error_strs: Vec<&str> = result.errors.iter().map(|s| s.as_str()).collect();
    let warning_strs: Vec<&str> = result.warnings.iter().map(|s| s.as_str()).collect();

    let exports_term = match &result.exports {
        Some(exports) => {
            let keys: Vec<Term<'a>> = exports.keys().map(|k| k.as_str().encode(env)).collect();
            let vals: Vec<Term<'a>> = exports.values().map(|v| v.name.as_str().encode(env)).collect();
            if keys.is_empty() {
                rustler::types::atom::nil().encode(env)
            } else {
                Term::map_from_arrays(env, &keys, &vals).unwrap()
            }
        }
        None => rustler::types::atom::nil().encode(env),
    };

    let map = Term::map_from_arrays(
        env,
        &[
            atoms::code().encode(env),
            atoms::css_vars().encode(env),
            atoms::errors().encode(env),
            atoms::warnings().encode(env),
            atoms::exports().encode(env),
        ],
        &[
            result.code.as_str().encode(env),
            css_vars.encode(env),
            error_strs.encode(env),
            warning_strs.encode(env),
            exports_term,
        ],
    )
    .unwrap();

    Ok((atoms::ok(), map).encode(env))
}

// ── CSS Bundling ──

#[rustler::nif(schedule = "DirtyCpu")]
fn bundle_css_nif<'a>(
    env: Env<'a>,
    entry_path: &str,
    minify: bool,
    chrome: i64,
    firefox: i64,
    safari: i64,
    css_modules: bool,
) -> NifResult<Term<'a>> {
    let targets = if chrome >= 0 || firefox >= 0 || safari >= 0 {
        Some(CssTargets {
            chrome: if chrome >= 0 { Some(chrome as u32) } else { None },
            firefox: if firefox >= 0 { Some(firefox as u32) } else { None },
            safari: if safari >= 0 { Some(safari as u32) } else { None },
            ..Default::default()
        })
    } else {
        None
    };

    let options = CssCompileOptions {
        minify,
        targets,
        css_modules,
        ..Default::default()
    };

    let result = bundle_css(entry_path, &options);

    let error_strs: Vec<&str> = result.errors.iter().map(|s| s.as_str()).collect();
    let warning_strs: Vec<&str> = result.warnings.iter().map(|s| s.as_str()).collect();

    let exports_term = match &result.exports {
        Some(exports) => {
            let keys: Vec<Term<'a>> = exports.keys().map(|k| k.as_str().encode(env)).collect();
            let vals: Vec<Term<'a>> = exports.values().map(|v| v.name.as_str().encode(env)).collect();
            if keys.is_empty() {
                rustler::types::atom::nil().encode(env)
            } else {
                Term::map_from_arrays(env, &keys, &vals).unwrap()
            }
        }
        None => rustler::types::atom::nil().encode(env),
    };

    let map = Term::map_from_arrays(
        env,
        &[
            atoms::code().encode(env),
            atoms::errors().encode(env),
            atoms::warnings().encode(env),
            atoms::exports().encode(env),
        ],
        &[
            result.code.as_str().encode(env),
            error_strs.encode(env),
            warning_strs.encode(env),
            exports_term,
        ],
    )
    .unwrap();

    Ok((atoms::ok(), map).encode(env))
}

// ── Vapor Split ──
// Produces {statics, slots} from Vapor IR, doing all HTML manipulation in Rust.

const PROP_MARKER: &str = "\x00PROP\x00";
const STRUCT_MARKER: &str = "\x00STRUCT\x00";
const TEXT_MARKER: &str = "\x00TEXT\x00";

/// Tag entry from HTML parsing: (position, tag_name, is_void_or_self_closing)
#[derive(Debug, Clone)]
struct TagEntry {
    pos: usize,
    name: std::string::String,
    parent: Option<usize>,
    child_index: usize,
    open_start: usize,
    open_end: usize,   // byte offset right after '>'
    close_start: Option<usize>, // byte offset of '</' or None for void/self-closing
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input"
        | "link" | "meta" | "param" | "source" | "track" | "wbr"
    )
}

fn parse_tag_tree(html: &str) -> std::vec::Vec<TagEntry> {
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut entries: std::vec::Vec<TagEntry> = std::vec::Vec::new();
    let mut stack: std::vec::Vec<usize> = std::vec::Vec::new(); // stack of tag positions
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' {
            if i + 1 < len && bytes[i + 1] == b'/' {
                // Closing tag — skip to '>'
                let start = i;
                i += 2;
                while i < len && bytes[i] != b'>' {
                    i += 1;
                }
                if i < len {
                    i += 1; // skip '>'
                }
                // Set close_start on matching stack entry and pop
                if let Some(top) = stack.pop() {
                    entries[top].close_start = Some(start);
                }
            } else {
                // Opening tag
                let open_start = i;
                i += 1;
                // Extract tag name
                let name_start = i;
                while i < len && bytes[i] != b' ' && bytes[i] != b'>' && bytes[i] != b'/' {
                    i += 1;
                }
                let tag_name = std::string::String::from_utf8_lossy(&bytes[name_start..i]).to_string();

                // Skip to end of tag
                let mut self_closing = false;
                while i < len && bytes[i] != b'>' {
                    if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'>' {
                        self_closing = true;
                    }
                    i += 1;
                }
                if i < len {
                    i += 1; // skip '>'
                }
                let open_end = i;

                let parent = stack.last().copied();
                let child_index = if let Some(p) = parent {
                    entries.iter().filter(|e| e.parent == Some(p)).count()
                } else {
                    entries.iter().filter(|e| e.parent.is_none()).count()
                };

                let pos = entries.len();
                entries.push(TagEntry {
                    pos,
                    name: tag_name.clone(),
                    parent,
                    child_index,
                    open_start,
                    open_end,
                    close_start: None,
                });

                if !self_closing && !is_void_element(&tag_name) {
                    stack.push(pos);
                }
            }
        } else {
            i += 1;
        }
    }

    entries
}

/// Build element_id → tag_position mapping
fn build_elem_to_tag(
    returns: &[usize],
    operations: &[OperationNode],
    tags: &[TagEntry],
) -> std::collections::HashMap<usize, usize> {
    let mut map = std::collections::HashMap::new();

    // Root elements map to their index position in the template
    for (idx, &elem_id) in returns.iter().enumerate() {
        // Find the idx-th root tag (parent == None)
        let root_tags: std::vec::Vec<_> = tags.iter().filter(|t| t.parent.is_none()).collect();
        if let Some(tag) = root_tags.get(idx) {
            map.insert(elem_id, tag.pos);
        }
    }

    // Resolve child_ref / next_ref iteratively
    loop {
        let prev_size = map.len();
        for op in operations {
            match op {
                OperationNode::ChildRef(node) => {
                    if let Some(&parent_tag_pos) = map.get(&node.parent_id) {
                        let children: std::vec::Vec<_> = tags
                            .iter()
                            .filter(|t| t.parent == Some(parent_tag_pos))
                            .collect();
                        if let Some(child) = children.get(node.offset) {
                            map.insert(node.child_id, child.pos);
                        }
                    }
                }
                OperationNode::NextRef(node) => {
                    if let Some(&prev_tag_pos) = map.get(&node.prev_id) {
                        if let Some(prev_entry) = tags.get(prev_tag_pos) {
                            let siblings: std::vec::Vec<_> = tags
                                .iter()
                                .filter(|t| t.parent == prev_entry.parent)
                                .collect();
                            let target_idx = prev_entry.child_index + node.offset;
                            if let Some(sibling) = siblings.get(target_idx) {
                                map.insert(node.child_id, sibling.pos);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if map.len() == prev_size {
            break;
        }
    }

    map
}

/// Inject an attribute string right before the '>' of the nth tag
fn inject_attr(html: &mut std::string::String, tags: &mut std::vec::Vec<TagEntry>, tag_pos: usize, attr: &str) {
    if let Some(entry) = tags.get(tag_pos) {
        // Insert right before the '>'
        let insert_at = entry.open_end - 1;
        html.insert_str(insert_at, attr);
        let delta = attr.len();
        // Update all offsets after insert point
        for e in tags.iter_mut() {
            if e.open_start > insert_at {
                e.open_start += delta;
            }
            if e.open_end > insert_at {
                e.open_end += delta;
            }
            if let Some(ref mut cs) = e.close_start {
                if *cs > insert_at {
                    *cs += delta;
                }
            }
        }
    }
}

/// Inject content before the closing tag of a given tag
fn inject_before_close(html: &mut std::string::String, tags: &mut std::vec::Vec<TagEntry>, tag_pos: usize, content: &str) {
    if let Some(entry) = tags.get(tag_pos) {
        let insert_at = if let Some(cs) = entry.close_start {
            cs
        } else {
            // Self-closing/void — insert right after opening tag
            entry.open_end
        };
        html.insert_str(insert_at, content);
        let delta = content.len();
        for e in tags.iter_mut() {
            if e.open_start > insert_at {
                e.open_start += delta;
            }
            if e.open_end > insert_at {
                e.open_end += delta;
            }
            if let Some(ref mut cs) = e.close_start {
                if *cs >= insert_at {
                    *cs += delta;
                }
            }
        }
    }
}

fn encode_slot_values<'a>(env: Env<'a>, kind: Term<'a>, values: Term<'a>) -> Term<'a> {
    Term::map_from_arrays(
        env,
        &[atoms::kind().encode(env), atoms::values().encode(env)],
        &[kind, values],
    ).unwrap()
}

fn encode_slot_value<'a>(env: Env<'a>, kind: Term<'a>, expr: &vize_atelier_core::SimpleExpressionNode) -> Term<'a> {
    Term::map_from_arrays(
        env,
        &[atoms::kind().encode(env), atoms::value().encode(env)],
        &[kind, encode_simple_expr(env, expr)],
    ).unwrap()
}

fn encode_split_block<'a, 'b>(
    env: Env<'a>,
    block: &'b BlockIRNode<'b>,
    ir: &'b RootIRNode<'b>,
) -> Term<'a> {
    let (statics, slots) = process_block(env, block, ir);
    let statics_term: std::vec::Vec<Term<'a>> = statics.iter().map(|s| s.as_str().encode(env)).collect();
    Term::map_from_arrays(
        env,
        &[atoms::statics().encode(env), atoms::slots().encode(env)],
        &[statics_term.encode(env), slots.encode(env)],
    ).unwrap()
}

fn encode_slot_if_split<'a, 'b>(
    env: Env<'a>,
    if_node: &'b IfIRNode<'b>,
    ir: &'b RootIRNode<'b>,
) -> Term<'a> {
    let positive_split = encode_split_block(env, &if_node.positive, ir);

    let negative_term = match &if_node.negative {
        Some(NegativeBranch::Block(block)) => encode_split_block(env, block, ir),
        Some(NegativeBranch::If(nested)) => encode_slot_if_split(env, nested, ir),
        None => rustler::types::atom::nil().encode(env),
    };

    Term::map_from_arrays(
        env,
        &[
            atoms::kind().encode(env),
            atoms::condition().encode(env),
            atoms::positive().encode(env),
            atoms::negative().encode(env),
        ],
        &[
            atoms::if_node().encode(env),
            encode_simple_expr(env, &if_node.condition),
            positive_split,
            negative_term,
        ],
    ).unwrap()
}

fn encode_slot_for_split<'a, 'b>(
    env: Env<'a>,
    for_node: &'b ForIRNode<'b>,
    ir: &'b RootIRNode<'b>,
) -> Term<'a> {
    let nil = rustler::types::atom::nil().encode(env);
    let render_split = encode_split_block(env, &for_node.render, ir);

    Term::map_from_arrays(
        env,
        &[
            atoms::kind().encode(env),
            atoms::source().encode(env),
            atoms::value().encode(env),
            atoms::key_prop().encode(env),
            atoms::render().encode(env),
        ],
        &[
            atoms::for_node().encode(env),
            encode_simple_expr(env, &for_node.source),
            for_node.value.as_ref().map(|v| encode_simple_expr(env, v)).unwrap_or(nil),
            for_node.key_prop.as_ref().map(|v| encode_simple_expr(env, v)).unwrap_or(nil),
            render_split,
        ],
    ).unwrap()
}

fn encode_slot_component<'a>(env: Env<'a>, node: &CreateComponentIRNode) -> Term<'a> {
    let props: std::vec::Vec<Term<'a>> = node.props.iter().map(|p| encode_ir_prop(env, p)).collect();
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
            atoms::value().encode(env),
        ],
        &[
            atoms::create_component().encode(env),
            node.tag.as_str().encode(env),
            props.encode(env),
            kind_atom.encode(env),
        ],
    ).unwrap()
}

fn process_block<'a, 'b>(
    env: Env<'a>,
    block: &'b BlockIRNode<'b>,
    ir: &'b RootIRNode<'b>,
) -> (std::vec::Vec<std::string::String>, std::vec::Vec<Term<'a>>) {
    // Resolve template HTML
    let template_html: std::string::String = block
        .returns
        .iter()
        .map(|&elem_id| {
            let template_idx = ir.element_template_map.get(&elem_id).copied().unwrap_or(elem_id);
            ir.templates.get(template_idx).map(|s| s.as_str()).unwrap_or("")
        })
        .collect();

    let mut html = template_html;
    let mut tags = parse_tag_tree(&html);
    let elem_to_tag = build_elem_to_tag(&block.returns, &block.operation, &tags);

    let mut slots: std::vec::Vec<Term<'a>> = std::vec::Vec::new();

    // Phase 0: Inject static event attributes
    for op in block.operation.iter() {
        if let OperationNode::SetEvent(ev) = op {
            if let Some(&tag_pos) = elem_to_tag.get(&ev.element) {
                let event_name = ev.key.content.as_str();
                let handler = ev.value.as_ref().map(|v| v.content.as_str()).unwrap_or(event_name);
                let attr = format!(" phx-{}=\"{}\"", event_name, handler);
                inject_attr(&mut html, &mut tags, tag_pos, &attr);
            }
        }
    }

    // Phase 1: Inject prop markers
    // Group effects by element and sort by element ID for deterministic order
    let all_effects: std::vec::Vec<_> = block.effect.iter().flat_map(|e| e.operations.iter()).collect();

    let mut prop_effects: std::vec::Vec<&SetPropIRNode> = std::vec::Vec::new();
    let mut text_effects: std::vec::Vec<&SetTextIRNode> = std::vec::Vec::new();
    let mut html_effects: std::vec::Vec<&SetHtmlIRNode> = std::vec::Vec::new();

    for op in &all_effects {
        match op {
            OperationNode::SetProp(p) => prop_effects.push(p),
            OperationNode::SetText(t) => text_effects.push(t),
            OperationNode::SetHtml(h) => html_effects.push(h),
            _ => {}
        }
    }

    // Sort props by element ID for deterministic ordering
    prop_effects.sort_by_key(|p| p.element);

    for prop in &prop_effects {
        if let Some(&tag_pos) = elem_to_tag.get(&prop.element) {
            let attr_name = prop.prop.key.content.as_str();
            let marker = format!(" {}=\"{}\"", attr_name, PROP_MARKER);
            inject_attr(&mut html, &mut tags, tag_pos, &marker);
            { let vals: std::vec::Vec<Term> = prop.prop.values.iter().map(|v| encode_simple_expr(env, v)).collect(); slots.push(encode_slot_values(env, atoms::set_prop().encode(env), vals.encode(env))); }
        }
    }

    // Phase 1b: Inject directive markers
    for op in block.operation.iter() {
        if let OperationNode::Directive(dir) = op {
            if let Some(&tag_pos) = elem_to_tag.get(&dir.element) {
                match dir.name.as_str() {
                    "vShow" => {
                        let marker = format!(" style=\"{}\"", PROP_MARKER);
                        inject_attr(&mut html, &mut tags, tag_pos, &marker);
                        if let Some(exp) = &dir.dir.exp {
                            match exp {
                                vize_atelier_core::ExpressionNode::Simple(s) => {
                                    slots.push(encode_slot_value(env, atoms::v_show().encode(env), s));
                                }
                                _ => {}
                            }
                        }
                    }
                    "model" => {
                        let value_marker = format!(" value=\"{}\"", PROP_MARKER);
                        inject_attr(&mut html, &mut tags, tag_pos, &value_marker);
                        if let Some(exp) = &dir.dir.exp {
                            match exp {
                                vize_atelier_core::ExpressionNode::Simple(s) => {
                                    slots.push(encode_slot_value(env, atoms::v_model().encode(env), s));
                                    // Also inject phx-change handler
                                    let handler_name = format!("{}_changed", s.content.as_str());
                                    let change_attr = format!(" phx-change=\"{}\"", handler_name);
                                    inject_attr(&mut html, &mut tags, tag_pos, &change_attr);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Phase 2: Replace text placeholders (the single space in templates)
    // Text effects target text nodes; the template has a single space as placeholder
    // We need to find these spaces between tags and replace with markers
    // Reparse after attr injections
    tags = parse_tag_tree(&html);

    for text in &text_effects {
        if let Some(&tag_pos) = elem_to_tag.get(&text.element) {
            // The text node is the content of this element
            // Find the content region (between open_end and close_start)
            if let Some(entry) = tags.get(tag_pos) {
                let content_start = entry.open_end;
                let content_end = entry.close_start.unwrap_or(content_start);
                let content = &html[content_start..content_end];

                // Replace the space with our text marker
                if content.contains(' ') {
                    let new_content = content.replacen(' ', TEXT_MARKER, 1);
                    html = format!("{}{}{}", &html[..content_start], new_content, &html[content_end..]);
                    tags = parse_tag_tree(&html);
                }
            }
        }
        { let vals: std::vec::Vec<Term> = text.values.iter().map(|v| encode_simple_expr(env, v)).collect(); slots.push(encode_slot_values(env, atoms::set_text().encode(env), vals.encode(env))); }
    }

    // Also handle set_html effects
    for h in &html_effects {
        if let Some(&tag_pos) = elem_to_tag.get(&h.element) {
            if let Some(entry) = tags.get(tag_pos) {
                let content_start = entry.open_end;
                let content_end = entry.close_start.unwrap_or(content_start);
                let content = &html[content_start..content_end];
                if content.contains(' ') {
                    let new_content = content.replacen(' ', TEXT_MARKER, 1);
                    html = format!("{}{}{}", &html[..content_start], new_content, &html[content_end..]);
                    tags = parse_tag_tree(&html);
                }
            }
        }
        slots.push(encode_slot_value(env, atoms::set_html().encode(env), &h.value));
    }

    // Phase 3: Inject structural markers (v-if, v-for, components)
    for op in block.operation.iter() {
        match op {
            OperationNode::If(if_node) => {
                if let Some(parent_id) = if_node.parent {
                    if let Some(&tag_pos) = elem_to_tag.get(&parent_id) {
                        inject_before_close(&mut html, &mut tags, tag_pos, STRUCT_MARKER);
                    }
                } else {
                    html.push_str(STRUCT_MARKER);
                }
                slots.push(encode_slot_if_split(env, if_node, ir));
            }
            OperationNode::For(for_node) => {
                if let Some(parent_id) = for_node.parent {
                    if let Some(&tag_pos) = elem_to_tag.get(&parent_id) {
                        inject_before_close(&mut html, &mut tags, tag_pos, STRUCT_MARKER);
                    }
                } else {
                    html.push_str(STRUCT_MARKER);
                }
                slots.push(encode_slot_for_split(env, for_node, ir));
            }
            OperationNode::CreateComponent(comp) => {
                if let Some(parent_id) = comp.parent {
                    if let Some(&tag_pos) = elem_to_tag.get(&parent_id) {
                        inject_before_close(&mut html, &mut tags, tag_pos, STRUCT_MARKER);
                    }
                } else {
                    html.push_str(STRUCT_MARKER);
                }
                slots.push(encode_slot_component(env, comp));
            }
            _ => {}
        }
    }

    // Phase 4: Split on markers
    let mut statics: std::vec::Vec<std::string::String> = std::vec::Vec::new();
    let mut current = std::string::String::new();

    let mut rest = html.as_str();
    loop {
        // Find the next marker
        let prop_pos = rest.find(PROP_MARKER);
        let text_pos = rest.find(TEXT_MARKER);
        let struct_pos = rest.find(STRUCT_MARKER);

        let next = [prop_pos, text_pos, struct_pos]
            .iter()
            .filter_map(|p| *p)
            .min();

        match next {
            None => {
                current.push_str(rest);
                break;
            }
            Some(pos) => {
                current.push_str(&rest[..pos]);
                statics.push(std::mem::take(&mut current));

                // Determine which marker and skip it
                if Some(pos) == prop_pos {
                    rest = &rest[pos + PROP_MARKER.len()..];
                } else if Some(pos) == text_pos {
                    rest = &rest[pos + TEXT_MARKER.len()..];
                } else {
                    rest = &rest[pos + STRUCT_MARKER.len()..];
                }
            }
        }
    }
    statics.push(current);

    (statics, slots)
}

#[rustler::nif(schedule = "DirtyCpu")]
fn vapor_split_nif<'a>(env: Env<'a>, source: &str) -> NifResult<Term<'a>> {
    let allocator = Bump::new();
    let parser_opts = ParserOptions::default();
    let (mut root, errors) = parse_with_options(&allocator, source, parser_opts);

    if !errors.is_empty() {
        let msgs: std::vec::Vec<std::string::String> = errors.iter().map(|e| e.message.to_string()).collect();
        return Ok((atoms::error(), msgs).encode(env));
    }

    let transform_opts = TransformOptions {
        vapor: true,
        ..Default::default()
    };
    transform(&allocator, &mut root, transform_opts, None);

    let ir = transform_to_ir(&allocator, &root);

    // Do everything inside this scope so ir is still alive when we encode Terms
    let (statics, slots) = process_block(env, &ir.block, &ir);

    let statics_term: std::vec::Vec<Term<'a>> = statics.iter().map(|s| s.as_str().encode(env)).collect();
    let templates: std::vec::Vec<&str> = ir.templates.iter().map(|s| s.as_str()).collect();
    let element_template_map: std::vec::Vec<(usize, usize)> = ir
        .element_template_map
        .iter()
        .map(|(&k, &v)| (k, v))
        .collect();

    let result = Term::map_from_arrays(
        env,
        &[
            atoms::statics().encode(env),
            atoms::slots().encode(env),
            atoms::templates().encode(env),
            atoms::element_template_map().encode(env),
        ],
        &[
            statics_term.encode(env),
            slots.encode(env),
            templates.encode(env),
            element_template_map.encode(env),
        ],
    )
    .unwrap();

    Ok((atoms::ok(), result).encode(env))
}

rustler::init!("Elixir.Vize.Native");
