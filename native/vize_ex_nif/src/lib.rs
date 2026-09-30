use lightningcss::dependencies::{Dependency, DependencyOptions};
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::{
    ParserFlags as CssParserFlags, ParserOptions as CssParserOptions, StyleSheet,
};
use rustler::{Atom, Encoder, Env, NifResult, Term};
use rustler_match_spec::{MatchEvent, Selector, ValueRef};
use vize_atelier_core::options::{
    CodegenMode, CodegenOptions, ParserOptions, TemplateSyntaxMode, TransformOptions,
};
use vize_atelier_core::parser::{parse, parse_with_options};
use vize_atelier_core::transform;
use vize_atelier_sfc::bundler::rewrite_template_asset_references;
use vize_atelier_sfc::compile_script::typescript::transform_typescript_to_js;
use vize_atelier_sfc::croquis::{analyze_sfc_descriptor, SfcCroquisOptions};
use vize_atelier_sfc::script::analyze_script_setup_to_summary;
use vize_atelier_sfc::{
    bundle_css, compile_css, compile_sfc_with_template_syntax_and_codegen_options, parse_css_ast,
    parse_sfc, print_css_ast, CssCompileOptions, CssTargets, SfcCompileOptions, SfcParseOptions,
};
use vize_atelier_sfc::{collect_template_asset_urls, generate_bundler_scope_id, TemplateAssetUrl};
use vize_atelier_ssr::compile_ssr;
use vize_atelier_vapor::{
    compile_vapor, compile_vapor_with_template_syntax_and_diagnostics, transform_to_ir,
    VaporCompilerOptions,
};
use vize_carton::{line_index::LineIndex, Allocator};

#[macro_use]
mod macros;
mod html_inject;
mod ir_encoding;

mod ir_encoders {
    use rustler::Encoder;
    use vize_atelier_vapor::ir::*;

    use crate::atoms;
    use crate::ir_encoding::{
        encode_directive_expression, encode_insertion_anchor, encode_simple_expr,
    };

    include!("generated_ir_encoders.rs");
}
mod term_encoding;
mod vapor_split;

use crate::term_encoding::{
    decode_json_value, error_term, ok_term, EncodedBinding, EncodedBundleCssResult,
    EncodedCompileSfcResult, EncodedComponentUsage, EncodedCssAstResult, EncodedCssCompileResult,
    EncodedDiagnostic, EncodedDts, EncodedEventListener, EncodedLintDiagnostic,
    EncodedParseSfcResult, EncodedPassedProp, EncodedPosition, EncodedPropDeclaration,
    EncodedRange, EncodedSass, EncodedSfcAnalysis, EncodedSfcStats, EncodedSourceLocation,
    EncodedSsrCompileResult, EncodedTemplateAsset, EncodedTemplateCompileResult,
    EncodedTemplateExpression, EncodedUndefinedRef, EncodedVaporDiagnosticsOutput, EncodedVaporIr,
    EncodedVaporOutput, EncodedVaporSplit,
};
use crate::vapor_split::process_block;

include!("generated_atoms.rs");
include!("generated_nifs.rs");

// ── SFC Parsing ──

fn parse_sfc_nif_impl<'a>(env: Env<'a>, source: &str) -> NifResult<Term<'a>> {
    let opts = SfcParseOptions::default();
    match parse_sfc(source, opts) {
        Ok(descriptor) => Ok(ok_term(
            env,
            EncodedParseSfcResult {
                descriptor: &descriptor,
            },
        )),
        Err(e) => Ok(error_term(env, format!("{e:?}"))),
    }
}

// ── SFC Analysis ──

fn analyze_sfc_nif_impl<'a>(env: Env<'a>, source: &str, mode: &str) -> NifResult<Term<'a>> {
    let parse_opts = SfcParseOptions {
        filename: "component.vue".into(),
        ..Default::default()
    };

    let descriptor = match parse_sfc(source, parse_opts) {
        Ok(descriptor) => descriptor,
        Err(error) => return Ok(error_term(env, error.message.as_str())),
    };

    let allocator = Allocator::new();
    let template_ast = descriptor.template.as_ref().map(|template| {
        let (root, _errors) = parse_with_options(
            &allocator,
            template.content.as_ref(),
            ParserOptions::default(),
        );
        root
    });

    let options = match mode {
        "lint" => SfcCroquisOptions::for_lint(),
        "compile" => SfcCroquisOptions::for_compile(),
        "declaration" => SfcCroquisOptions::for_declaration(),
        _ => SfcCroquisOptions::full(),
    };

    let croquis = analyze_sfc_descriptor(&descriptor, template_ast.as_ref(), options);
    let stats = croquis.stats();

    let analysis = EncodedSfcAnalysis {
        stats: EncodedSfcStats {
            bindings: stats.binding_count,
            props: stats.prop_count,
            emits: stats.emit_count,
            models: stats.model_count,
            used_components: stats.used_components,
            used_directives: stats.used_directives,
            undefined_refs: stats.undefined_ref_count,
        },
        bindings: croquis
            .bindings
            .iter()
            .map(|(name, binding_type)| EncodedBinding {
                name: name.to_string(),
                kind: format!("{:?}", binding_type),
            })
            .collect(),
        props: croquis
            .get_props()
            .map(|(name, required)| EncodedPropDeclaration {
                name: name.to_string(),
                required,
            })
            .collect(),
        emits: croquis.get_emits().map(ToString::to_string).collect(),
        models: croquis.get_models().map(ToString::to_string).collect(),
        used_components: croquis
            .used_components
            .iter()
            .map(ToString::to_string)
            .collect(),
        used_directives: croquis
            .used_directives
            .iter()
            .map(ToString::to_string)
            .collect(),
        undefined_refs: croquis
            .undefined_refs
            .iter()
            .map(|reference| EncodedUndefinedRef {
                name: reference.name.to_string(),
                offset: reference.offset,
                context: reference.context.to_string(),
            })
            .collect(),
        component_usages: croquis
            .component_usages
            .iter()
            .map(|usage| EncodedComponentUsage {
                name: usage.name.to_string(),
                props: usage
                    .props
                    .iter()
                    .map(|prop| EncodedPassedProp {
                        name: prop.name.to_string(),
                        value: prop.value.as_ref().map(ToString::to_string),
                        is_dynamic: prop.is_dynamic,
                    })
                    .collect(),
                events: usage
                    .events
                    .iter()
                    .map(|event| EncodedEventListener {
                        name: event.name.to_string(),
                        handler: event.handler.as_ref().map(ToString::to_string),
                    })
                    .collect(),
                has_spread_attrs: usage.has_spread_attrs,
            })
            .collect(),
        template_expressions: croquis
            .template_expressions
            .iter()
            .map(|expression| EncodedTemplateExpression {
                source: expression.content.to_string(),
                kind: expression.kind.as_str().to_owned(),
                range: EncodedRange {
                    start: expression.start,
                    end: expression.end,
                },
            })
            .collect(),
    };

    Ok(ok_term(env, analysis))
}

// ── SFC Compilation ──

#[allow(clippy::too_many_arguments)]
fn compile_sfc_nif_impl<'a>(
    env: Env<'a>,
    source: &str,
    filename: &str,
    scope_id: &str,
    vapor: bool,
    ssr: bool,
    custom_renderer: bool,
    strip_types: bool,
    source_map: bool,
) -> NifResult<Term<'a>> {
    let mut parse_opts = SfcParseOptions::default();
    if !filename.is_empty() {
        parse_opts.filename = filename.into();
    }

    let descriptor = match parse_sfc(source, parse_opts) {
        Ok(d) => d,
        Err(e) => return Ok(error_term(env, format!("{e:?}"))),
    };

    let mut compile_opts = SfcCompileOptions {
        vapor,
        template: vize_atelier_sfc::TemplateCompileOptions {
            ssr,
            custom_renderer,
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

    let codegen_opts = CodegenOptions {
        source_map,
        ..Default::default()
    };

    match compile_sfc_with_template_syntax_and_codegen_options(
        &descriptor,
        compile_opts,
        TemplateSyntaxMode::Standard,
        codegen_opts,
    ) {
        Ok(result) => {
            let stripped = strip_types.then(|| transform_typescript_to_js(result.code.as_str()));
            let code_override = stripped.as_deref();
            let source_map = source_map
                .then_some(result.map.as_ref())
                .flatten()
                .and_then(|map| serde_json::to_string(map).ok())
                .map(Into::into);

            Ok(ok_term(
                env,
                EncodedCompileSfcResult {
                    result: &result,
                    descriptor: &descriptor,
                    code_override,
                    source_map,
                    template_hash: descriptor.template_hash(),
                    style_hash: descriptor.style_hash(),
                    script_hash: descriptor.script_hash(),
                },
            ))
        }
        Err(e) => Ok(error_term(env, e.message.as_str())),
    }
}

// ── SFC Bundler Helpers ──

fn sfc_template_assets_nif_impl<'a>(
    env: Env<'a>,
    source: &str,
    filename: &str,
) -> NifResult<Term<'a>> {
    let assets =
        collect_template_asset_urls(source, None, (!filename.is_empty()).then_some(filename));
    let assets: Vec<EncodedTemplateAsset> = assets
        .iter()
        .map(|asset| EncodedTemplateAsset {
            url: asset.url.to_string(),
            var_name: asset.var_name.to_string(),
        })
        .collect();
    Ok(assets.encode(env))
}

fn rewrite_sfc_template_assets_nif_impl<'a>(
    env: Env<'a>,
    code: &str,
    assets: Vec<(String, String)>,
) -> NifResult<Term<'a>> {
    let assets: Vec<TemplateAssetUrl> = assets
        .into_iter()
        .map(|(url, var_name)| TemplateAssetUrl {
            url: url.into(),
            var_name: var_name.into(),
        })
        .collect();
    Ok(rewrite_template_asset_references(code, &assets).encode(env))
}

fn sfc_scope_id_nif_impl<'a>(
    env: Env<'a>,
    filename: &str,
    root: &str,
    production: bool,
    source: &str,
) -> NifResult<Term<'a>> {
    let scope_id = generate_bundler_scope_id(
        filename,
        (!root.is_empty()).then_some(root),
        production,
        (!source.is_empty()).then_some(source),
    );
    Ok(scope_id.encode(env))
}

// ── Template Compilation ──

fn compile_template_nif_impl<'a>(
    env: Env<'a>,
    source: &str,
    mode: &str,
    ssr: bool,
) -> NifResult<Term<'a>> {
    let allocator = Allocator::new();
    let (mut root, errors) = parse(&allocator, source);

    if !errors.is_empty() {
        let msgs: Vec<std::string::String> = errors.iter().map(|e| e.message.to_string()).collect();
        return Ok(error_term(env, msgs));
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

    Ok(ok_term(
        env,
        EncodedTemplateCompileResult {
            code: result.code.as_str(),
            preamble: result.preamble.as_str(),
            helpers,
        },
    ))
}

// ── SSR Compilation ──

fn compile_ssr_nif_impl<'a>(env: Env<'a>, source: &str) -> NifResult<Term<'a>> {
    let allocator = Allocator::new();
    let (_root, errors, result) = compile_ssr(&allocator, source);

    if !errors.is_empty() {
        let msgs: Vec<std::string::String> = errors.iter().map(|e| e.message.to_string()).collect();
        return Ok(error_term(env, msgs));
    }

    Ok(ok_term(
        env,
        EncodedSsrCompileResult {
            code: result.code.as_str(),
            preamble: result.preamble.as_str(),
        },
    ))
}

// ── Vapor Compilation ──

fn position(line_index: &LineIndex<'_>, offset: u32) -> EncodedPosition {
    let (line, column) = line_index.line_col(offset as usize);
    EncodedPosition {
        offset,
        line: line + 1,
        column: column + 1,
    }
}

fn source_location(
    location: &vize_atelier_core::SourceLocation,
    source: &str,
    line_index: &LineIndex<'_>,
) -> EncodedSourceLocation {
    EncodedSourceLocation {
        start: position(line_index, location.span.start),
        end: position(line_index, location.span.end),
        source: location.span.slice(source).to_owned(),
    }
}

fn compiler_diagnostic(
    error: &vize_atelier_core::CompilerError,
    source: &str,
    line_index: &LineIndex<'_>,
) -> EncodedDiagnostic {
    EncodedDiagnostic {
        code: Some(format!("{:?}", error.code)),
        message: error.message.to_string(),
        recoverable: error.is_recoverable(),
        location: error
            .loc
            .as_ref()
            .map(|location| source_location(location, source, line_index)),
    }
}

fn template_syntax_mode(value: &str) -> TemplateSyntaxMode {
    match value {
        "quirks" => TemplateSyntaxMode::Quirks,
        _ => TemplateSyntaxMode::Standard,
    }
}

fn compile_vapor_nif_impl<'a>(
    env: Env<'a>,
    source: &str,
    ssr: bool,
    diagnostics: bool,
    template_syntax: &str,
) -> NifResult<Term<'a>> {
    let allocator = Allocator::new();
    let opts = VaporCompilerOptions {
        ssr,
        ..Default::default()
    };
    let syntax = template_syntax_mode(template_syntax);

    if diagnostics || syntax.is_quirks() {
        let (result, parser_diagnostics) =
            compile_vapor_with_template_syntax_and_diagnostics(&allocator, source, opts, syntax);

        let line_index = LineIndex::new(source);
        let mut diagnostics: Vec<EncodedDiagnostic> = parser_diagnostics
            .iter()
            .map(|diagnostic| compiler_diagnostic(diagnostic, source, &line_index))
            .collect();

        if !result.error_messages.is_empty() {
            diagnostics.extend(
                result
                    .error_messages
                    .iter()
                    .map(|message| EncodedDiagnostic {
                        code: None,
                        message: message.to_string(),
                        recoverable: false,
                        location: None,
                    }),
            );
            return Ok(error_term(env, diagnostics));
        }

        let output = EncodedVaporDiagnosticsOutput {
            code: result.code.to_string(),
            templates: result.templates.iter().map(ToString::to_string).collect(),
            diagnostics,
        };
        return Ok(ok_term(env, output));
    }

    let result = compile_vapor(&allocator, source, opts);

    if !result.error_messages.is_empty() {
        let msgs: Vec<&str> = result.error_messages.iter().map(|s| s.as_str()).collect();
        return Ok(error_term(env, msgs));
    }

    let output = EncodedVaporOutput {
        code: result.code.to_string(),
        templates: result.templates.iter().map(ToString::to_string).collect(),
    };
    Ok(ok_term(env, output))
}

// ── Vapor IR ──

fn vapor_ir_nif_impl<'a>(env: Env<'a>, source: &str) -> NifResult<Term<'a>> {
    let allocator = Allocator::new();
    let parser_opts = ParserOptions::default();
    let (mut root, errors) = parse_with_options(&allocator, source, parser_opts);

    if !errors.is_empty() {
        let msgs: Vec<std::string::String> = errors.iter().map(|e| e.message.to_string()).collect();
        return Ok(error_term(env, msgs));
    }

    let transform_opts = TransformOptions {
        vapor: true,
        ..Default::default()
    };
    transform(&allocator, &mut root, transform_opts, None);

    let ir = transform_to_ir(&allocator, &root, source);

    let output = EncodedVaporIr {
        templates: ir.templates.iter().map(ToString::to_string).collect(),
        components: ir.component.iter().map(ToString::to_string).collect(),
        directives: ir.directive.iter().map(ToString::to_string).collect(),
        block: ir_encoders::encode_block_ir_node(env, &ir.block),
        element_template_map: ir
            .element_template_map
            .iter()
            .map(|(&element, &template)| (element, template))
            .collect(),
    };

    Ok(ok_term(env, output))
}

// ── Linting ──

fn lint_nif_impl<'a>(env: Env<'a>, source: &str, filename: &str) -> NifResult<Term<'a>> {
    use vize_patina::Linter;

    let linter = Linter::default();
    let result = linter.lint_sfc(source, filename);
    let diagnostics: Vec<Term<'a>> = result
        .diagnostics
        .iter()
        .map(|d| {
            EncodedLintDiagnostic {
                message: d.message.as_str(),
                name: d.rule_name,
            }
            .encode(env)
        })
        .collect();

    Ok(ok_term(env, diagnostics))
}

// ── CSS Compilation ──

fn compile_sass_nif_impl<'a>(
    env: Env<'a>,
    source: &str,
    syntax: &str,
    filename: &str,
    load_paths: Vec<String>,
    compressed: bool,
) -> NifResult<Term<'a>> {
    let input_syntax = match syntax {
        "sass" => grass::InputSyntax::Sass,
        "scss" => grass::InputSyntax::Scss,
        _ => return Ok(error_term(env, format!("Unknown Sass syntax: {syntax}"))),
    };

    let mut options = grass::Options::default()
        .input_syntax(input_syntax)
        .style(if compressed {
            grass::OutputStyle::Compressed
        } else {
            grass::OutputStyle::Expanded
        });

    if !filename.is_empty() {
        if let Some(parent) = std::path::Path::new(filename).parent() {
            options = options.load_path(parent);
        }
    }

    for path in load_paths {
        options = options.load_path(path);
    }

    match grass::from_string(source.to_owned(), &options) {
        Ok(code) => Ok(ok_term(env, EncodedSass { code })),
        Err(error) => Ok(error_term(env, error.to_string())),
    }
}

fn css_targets(chrome: i64, firefox: i64, safari: i64) -> Option<CssTargets> {
    if chrome >= 0 || firefox >= 0 || safari >= 0 {
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
    }
}

fn css_parser_options<'a>(
    filename: &'a str,
    custom_media: bool,
    css_modules: bool,
) -> CssParserOptions<'a> {
    let mut flags = CssParserFlags::NESTING | CssParserFlags::DEEP_SELECTOR_COMBINATOR;
    if custom_media {
        flags |= CssParserFlags::CUSTOM_MEDIA;
    }

    let css_modules_config = if css_modules {
        Some(lightningcss::css_modules::Config {
            pattern: lightningcss::css_modules::Pattern::default(),
            ..Default::default()
        })
    } else {
        None
    };

    CssParserOptions {
        filename: if filename.is_empty() {
            "style.css".into()
        } else {
            filename.into()
        },
        flags,
        css_modules: css_modules_config,
        ..Default::default()
    }
}

fn find_line_bounds(source: &str, line: u32) -> Option<(usize, usize)> {
    let mut current_line = 1_u32;
    let mut line_start = 0_usize;

    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            if current_line == line {
                return Some((line_start, index));
            }
            current_line += 1;
            line_start = index + 1;
        }
    }

    if current_line == line {
        Some((line_start, source.len()))
    } else {
        None
    }
}

fn find_url_range(source: &str, line: u32, column: u32, url: &str) -> Option<(usize, usize)> {
    let (line_start, line_end) = find_line_bounds(source, line)?;
    let line_source = &source[line_start..line_end];
    let target_column = column as usize;

    let match_start = line_source
        .match_indices(url)
        .map(|(index, _)| index)
        .min_by_key(|index| index.abs_diff(target_column))?;

    let start = line_start + match_start;
    Some((start, start + url.len()))
}

struct CssDependencyEvent {
    tag: Atom,
    url: String,
    start: usize,
    end: usize,
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
    supports: Option<String>,
    media: Option<String>,
}

impl<'a> MatchEvent<'a> for &'a CssDependencyEvent {
    fn tag(&self) -> Atom {
        self.tag
    }

    fn arity(&self) -> usize {
        10
    }

    fn positional_field(&self, index: usize) -> Option<ValueRef<'a>> {
        match index {
            1 => Some(ValueRef::Str(self.url.as_str())),
            2 => Some(ValueRef::U64(self.start as u64)),
            3 => Some(ValueRef::U64(self.end as u64)),
            4 => Some(ValueRef::U64(self.start_line.into())),
            5 => Some(ValueRef::U64(self.start_column.into())),
            6 => Some(ValueRef::U64(self.end_line.into())),
            7 => Some(ValueRef::U64(self.end_column.into())),
            8 => Some(optional_string(self.supports.as_deref())),
            9 => Some(optional_string(self.media.as_deref())),
            _ => None,
        }
    }

    fn field(&self, name: Atom) -> Option<ValueRef<'a>> {
        if name == atoms::url() {
            Some(ValueRef::Str(self.url.as_str()))
        } else if name == atoms::start() {
            Some(ValueRef::U64(self.start as u64))
        } else if name == atoms::end_() {
            Some(ValueRef::U64(self.end as u64))
        } else if name == atoms::start_line() {
            Some(ValueRef::U64(self.start_line.into()))
        } else if name == atoms::start_column() {
            Some(ValueRef::U64(self.start_column.into()))
        } else if name == atoms::end_line() {
            Some(ValueRef::U64(self.end_line.into()))
        } else if name == atoms::end_column() {
            Some(ValueRef::U64(self.end_column.into()))
        } else if name == atoms::supports() {
            Some(optional_string(self.supports.as_deref()))
        } else if name == atoms::media() {
            Some(optional_string(self.media.as_deref()))
        } else {
            None
        }
    }
}

fn optional_string(value: Option<&str>) -> ValueRef<'_> {
    match value {
        Some(value) => ValueRef::Str(value),
        None => ValueRef::Atom(rustler::types::atom::nil()),
    }
}

fn select_css_nif_impl<'a>(
    env: Env<'a>,
    source: &str,
    filename: &str,
    custom_media: bool,
    css_modules: bool,
    selector_term: Term<'a>,
) -> NifResult<Term<'a>> {
    let selector = Selector::from_term(selector_term)?;

    let stylesheet = match StyleSheet::parse(
        source,
        css_parser_options(filename, custom_media, css_modules),
    ) {
        Ok(stylesheet) => stylesheet,
        Err(error) => return Ok(error_term(env, vec![format!("CSS parse error: {error}")])),
    };

    let result = match stylesheet.to_css(PrinterOptions {
        analyze_dependencies: Some(DependencyOptions {
            remove_imports: false,
        }),
        ..Default::default()
    }) {
        Ok(result) => result,
        Err(error) => return Ok(error_term(env, vec![format!("CSS print error: {error:?}")])),
    };

    let mut events = Vec::new();

    for dependency in result.dependencies.unwrap_or_default() {
        match dependency {
            Dependency::Url(dependency) => {
                let Some((start, end)) = find_url_range(
                    source,
                    dependency.loc.start.line,
                    dependency.loc.start.column,
                    &dependency.url,
                ) else {
                    return Ok(error_term(
                        env,
                        vec![format!(
                            "Could not locate CSS URL range for {}",
                            dependency.url
                        )],
                    ));
                };

                events.push(CssDependencyEvent {
                    tag: atoms::css_url(),
                    url: dependency.url.to_string(),
                    start,
                    end,
                    start_line: dependency.loc.start.line,
                    start_column: dependency.loc.start.column,
                    end_line: dependency.loc.end.line,
                    end_column: dependency.loc.end.column,
                    supports: None,
                    media: None,
                });
            }
            Dependency::Import(dependency) => {
                let Some((start, end)) = find_url_range(
                    source,
                    dependency.loc.start.line,
                    dependency.loc.start.column,
                    &dependency.url,
                ) else {
                    return Ok(error_term(
                        env,
                        vec![format!(
                            "Could not locate CSS import range for {}",
                            dependency.url
                        )],
                    ));
                };

                events.push(CssDependencyEvent {
                    tag: atoms::css_import(),
                    url: dependency.url.to_string(),
                    start,
                    end,
                    start_line: dependency.loc.start.line,
                    start_column: dependency.loc.start.column,
                    end_line: dependency.loc.end.line,
                    end_column: dependency.loc.end.column,
                    supports: dependency.supports,
                    media: dependency.media,
                });
            }
        }
    }

    let mut urls = Vec::new();

    for event in &events {
        selector.run_event(env, &event, &mut urls)?;
    }

    Ok(ok_term(env, urls))
}

fn parse_css_ast_nif_impl<'a>(
    env: Env<'a>,
    source: &str,
    filename: &str,
    custom_media: bool,
    css_modules: bool,
) -> NifResult<Term<'a>> {
    let options = CssCompileOptions {
        filename: if filename.is_empty() {
            None
        } else {
            Some(filename.into())
        },
        custom_media,
        css_modules,
        ..Default::default()
    };

    let result = parse_css_ast(source, &options);

    Ok(ok_term(env, EncodedCssAstResult { result: &result }))
}

fn print_css_ast_nif_impl<'a>(
    env: Env<'a>,
    ast: Term<'a>,
    minify: bool,
    chrome: i64,
    firefox: i64,
    safari: i64,
) -> NifResult<Term<'a>> {
    let ast = match decode_json_value(ast) {
        Ok(ast) => ast,
        Err(_) => {
            let result = vize_atelier_sfc::CssCompileResult {
                code: Default::default(),
                map: None,
                css_vars: vec![],
                errors: vec!["Invalid CSS AST term".into()],
                warnings: vec![],
                exports: None,
            };

            return Ok(ok_term(env, EncodedCssCompileResult { result: &result }));
        }
    };

    let options = CssCompileOptions {
        minify,
        targets: css_targets(chrome, firefox, safari),
        ..Default::default()
    };

    let result = print_css_ast(ast, &options);

    Ok(ok_term(env, EncodedCssCompileResult { result: &result }))
}

#[allow(clippy::too_many_arguments)]
fn compile_css_nif_impl<'a>(
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

    Ok(ok_term(env, EncodedCssCompileResult { result: &result }))
}

// ── CSS Bundling ──

fn bundle_css_nif_impl<'a>(
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
        minify,
        targets,
        css_modules,
        ..Default::default()
    };

    let result = bundle_css(entry_path, &options);

    Ok(ok_term(env, EncodedBundleCssResult { result: &result }))
}

fn vapor_split_nif_impl<'a>(env: Env<'a>, source: &str) -> NifResult<Term<'a>> {
    let allocator = Allocator::new();
    let parser_opts = ParserOptions::default();
    let (mut root, errors) = parse_with_options(&allocator, source, parser_opts);

    if !errors.is_empty() {
        let msgs: std::vec::Vec<std::string::String> =
            errors.iter().map(|e| e.message.to_string()).collect();
        return Ok(error_term(env, msgs));
    }

    let transform_opts = TransformOptions {
        vapor: true,
        ..Default::default()
    };
    transform(&allocator, &mut root, transform_opts, None);

    let ir = transform_to_ir(&allocator, &root, source);

    let (statics, slots) = process_block(env, &ir.block, &ir, source);

    let split = EncodedVaporSplit {
        statics,
        slots,
        templates: ir.templates.iter().map(ToString::to_string).collect(),
        element_template_map: ir
            .element_template_map
            .iter()
            .map(|(&element, &template)| (element, template))
            .collect(),
    };
    Ok(ok_term(env, split))
}

// ── Declaration .d.ts Generation ──

fn generate_dts_nif_impl<'a>(env: Env<'a>, source: &str, filename: &str) -> NifResult<Term<'a>> {
    let parse_opts = SfcParseOptions {
        filename: if filename.is_empty() {
            "component.vue".into()
        } else {
            filename.into()
        },
        ..Default::default()
    };

    let descriptor = match parse_sfc(source, parse_opts) {
        Ok(d) => d,
        Err(e) => return Ok(error_term(env, format!("{e:?}"))),
    };

    let plain_script = descriptor.script.as_ref().map(|s| s.content.as_ref());
    let setup_script = descriptor.script_setup.as_ref().map(|s| s.content.as_ref());

    let summary = match setup_script {
        Some(content) => analyze_script_setup_to_summary(content),
        None => vize_croquis::Croquis::new(),
    };

    let output = match (plain_script, setup_script) {
        (Some(plain), Some(setup)) => {
            vize_croquis::declaration_ts::generate_declaration_ts_with_split_scripts(
                &summary, plain, setup,
            )
        }
        (_, Some(setup)) => {
            vize_croquis::declaration_ts::generate_declaration_ts(&summary, Some(setup))
        }
        (Some(plain), None) => {
            vize_croquis::declaration_ts::generate_declaration_ts(&summary, Some(plain))
        }
        (None, None) => vize_croquis::declaration_ts::generate_declaration_ts(&summary, None),
    };

    let dts = EncodedDts {
        dts: output.content.to_string(),
    };
    Ok(ok_term(env, dts))
}

rustler::init!("Elixir.Vize.Native");
