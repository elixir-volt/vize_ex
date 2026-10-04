# Changelog

## Unreleased

### Changed

- Build on Vize 0.431.0, which fixes three bugs found through vize_ex:
  - Static attribute values are decoded once in Vapor templates ([vize#7502](https://github.com/ubugeeei-prod/vize/issues/7502)). `&amp;lt;` used to become `<`.
  - Implicit default slot content is kept beside a named `<template #name>` in `compile_vapor` ([vize#7570](https://github.com/ubugeeei-prod/vize/issues/7570)).
  - `vapor_ir/1` keeps a static `style` merged with `:style` ([vize#7600](https://github.com/ubugeeei-prod/vize/issues/7600)).

## 0.17.0 - 2026-10-03

### Breaking changes

- `Vize.vapor_split/1` is replaced by `Vize.split_template/2`, which works from Vize's L2 semantic IR, the representation Vize's SSR compiler builds on, instead of scanning Vapor's template HTML. The split no longer has to locate elements in generated HTML, so whole classes of bugs are gone: slots dropped after text nodes, components placed before their siblings, a root `v-if` rendered twice, and implicit default slot content lost beside a named slot. The slots have a new shape, documented in [Splitting templates](guides/features/templates-and-ssr.md#splitting-templates):
  - Expressions are source strings, and every slot has a `:position`.
  - An `:attr` slot is a whole attribute, so a renderer can leave it out, as Vue does for `null` and false boolean attributes; a static attribute of the same name comes with it.
  - Components carry their props, events, and the content of each slot; `<slot>` outlets are slots too.
  - `v-show`, `v-model` on `<input>` and `<textarea>`, `v-html`, `v-text`, `v-bind` objects and dynamic attribute names are slots.
  - Events and `v-model`s are `:on` and `:model` bindings.
- Errors from `split_template/2` are `Vize.Error`s with a `Vize.Diagnostic` and position for each problem, such as an expression that doesn't parse; it used to return a list of strings.

### Added

- `split_template/2` takes `root_attrs: true` for a component's template: the attributes of a single root element come back as one `:root_attrs` slot, so a caller can merge the [fallthrough attributes](https://vuejs.org/guide/components/attrs.html) a parent passes.
- Warnings, such as a custom directive that doesn't run on the server, come back in the split's `:diagnostics`.
- `Vize.Diagnostic` has a `:severity`, and `Vize.Diagnostic.to_code_diagnostic/2` converts one to Elixir's `t:Code.diagnostic/1` shape, positioned in the file a template came from.
- `Vize.Error` messages list their diagnostics with positions.

## 0.16.1 - 2026-10-03

### Fixed

- `vapor_split/1` no longer drops slots when a text node comes before an element. Vapor counts text nodes when it navigates the template, so in `<li>a<ul><li>{{ b }}</li></ul></li>` the split looked for the wrong element and silently left `{{ b }}` out. Elements are now located by their DOM node position.
- `vapor_split/1` returns an error if it can't locate an element a slot or binding belongs to, instead of leaving the slot out.

## 0.16.0 - 2026-10-01

### Added

- SFC block locations (`loc` in `parse_sfc/1` results) include `tag_start` and `tag_end`, the span of the block's tags. Their encoder is now generated from Vize's `BlockLocation`, which the handwritten one had fallen behind.

### Changed

- **Breaking:** `vapor_split/1` no longer writes LiveView attributes. Events and `v-model`s are returned in a new `:bindings` list instead of becoming `phx-*` attributes and a `phx-change="<name>_changed"` convention. Each binding carries the Vapor IR node and the `{static_index, offset}` where its element's start tag ends, so callers such as PhoenixVapor add their own attributes. Every nested split has its own `:bindings`.
- NIFs take one typed options map instead of positional arguments; `compile_sfc_nif` had eight. RustQ derives the Rust struct and decoder for each map from a typespec, and the public functions keep their keyword options and defaults. CSS browser targets are optional versions instead of `-1` sentinels.

### Fixed

- Document that CSS `:targets` take effect only with `minify: true`, because Vize applies them while minifying.

## 0.15.0 - 2026-09-30

### Added

- Encode the new `set_merged_props` Vapor IR operation in `vapor_ir/1`, with `:object` and `:group` sources that carry their value under `:value`.
- Report `:transition` and `:transition_group` component kinds in `vapor_ir/1` and `vapor_split/1`.

### Changed

- Upgrade the upstream Vize workspace from 0.362 to 0.429.1. Vapor output now registers delegated events with `delegateEvents(...)`, so native `@click` handlers on plain elements fire again, and hydrates Vue 3.6.0-rc.9 SSR markup: it no longer passes a sibling index to `next()` and creates components and slots in document order.
- `vapor_ir/1` encodes a component, `v-if`, or `v-for` insertion anchor as the placeholder node id (as before) or, when the block is appended, as `{:index, n}` with its hydration start unit.
- Build SFC source maps through upstream codegen options instead of the removed `build_sfc_source_map`.
- Generate the `vapor_ir/1` encoders from `vize_atelier_vapor`'s Rust source with RustQ instead of maintaining them by hand. Every existing key and value is unchanged. The output now also includes the IR fields the handwritten encoders dropped, such as component `slots`, `v_show`, and `is_expr`, event `modifiers`, directive `builtin` and `input_type`, slot outlet `fallback`, node `id`s, `dynamic` info, and prop `value_kind`.
- `vapor_split/1` component props include `value_kind`.
- Derive the NIF result maps (`analyze_sfc`, `compile_vapor`, `vapor_ir`, `vapor_split` and its slots, template assets, `generate_dts`, and Sass) from Elixir typespecs instead of building them by hand. Their keys and values are unchanged, except that `compile_vapor/2` compile errors in a diagnostics list now carry `code: nil` and `location: nil`, like parser diagnostics.

### Fixed

- Keep `v-html` in `vapor_split/1`. The element's template content is empty, so the slot marker had no placeholder to replace and the `set_html` slot was dropped; the rendered element stayed empty.
- Replace Vapor's `<!---->` anchor comments with structural slots in `vapor_split/1`, so they no longer leak into statics and element offsets after an anchor stay aligned.
- Replace the whole text node for mixed static and dynamic text in `vapor_split/1`. Previously the static parts were rendered twice, e.g. `Hello {{ name }}` produced `HelloHello Ada `.

### Compatibility

- Declare `rust-version = "1.95"` for the NIF crate, so building from source with an older toolchain fails with a clear error.

## 0.14.2 - 2026-08-24

### Added

- Add optional SFC source maps and expose scoped-style, style block, and custom block metadata in compile results.
- Add `Vize.SFC.collect_template_assets/2`, `Vize.SFC.rewrite_asset_references/2`, `Vize.SFC.external_sources/1`, and `Vize.SFC.scope_id/2` for bundler integrations.
- Include external block sources and SFC descriptor metadata in `parse_sfc/1` results.

### Changed

- Upgrade the upstream Vize workspace from 0.290 to 0.362 and consume its published crates instead of the monorepo git revision.

### Fixed

- Preserve document order when pairing Vapor statics with nested property, text, and structural slots.

### Compatibility

- Require Rust 1.95 or later when compiling the NIF from source.

## 0.14.1 - 2026-07-20

### Added

- Add an x86_64 Windows precompiled NIF target.

## 0.14.0 - 2026-07-10

### Added

- Add native Sass and SCSS compilation through `Vize.CSS.compile_sass/2` and `Vize.CSS.compile_sass!/2`, backed by the Rust `grass` compiler. Supports SCSS and indented Sass syntax, relative imports, additional load paths, and compressed output.

### Changed

- Upgrade the upstream Vize workspace from 0.206 to 0.290.

## 0.13.1 - 2026-06-23

### Added

- Add `:imports` and `:dependencies` selectors to `Vize.CSS.select/3` for parser-backed CSS `@import` dependency discovery.

## 0.13.0 - 2026-06-19

### Added

- Add `Vize.CSS.select/3` for compact parser-backed CSS event selection.
- Add the `:urls` CSS selector for `url()` references with source byte ranges and locations.

### Changed

- `Vize.CSS.collect_urls/2` now uses the CSS selector API internally.
- Native CSS selection now uses the shared `rustler_match_spec` crate.
- Upgrade native Rustler dependency to 0.38.
- Struct constructors now use strict atom-keyed input contracts.

## 0.12.0

### Added

- Add `Vize.CSS.collect_urls/2` and `Vize.CSS.rewrite_urls/3` for parser-backed CSS URL source rewriting without CSS AST print round-trips.
- Add bang variants for CSS URL helpers.
- Add structured `Vize.Error`, `Vize.Diagnostic`, source range/location, CSS URL, Vapor result, and Croquis structs.
- Add structured Vapor diagnostics with `diagnostics: true` and `template_syntax: :standard | :quirks` options.
- Add `Vize.analyze_sfc/2` and `Vize.analyze_sfc!/2` returning `%Vize.Croquis{}` semantic summaries.

### Changed

- Bump upstream Vize crates 0.112 → 0.206.
- `Vize.compile_vapor/2` now returns `%Vize.Vapor.Result{}`.

## 0.11.1

- Bump upstream Vize crates 0.109 → 0.112.
- Fix CSS AST print round-trip for `image-set(...)`.

## 0.11.0

### Breaking

- Bump upstream Vize crates 0.76 → 0.109.
- CSS APIs are namespaced under `Vize.CSS`; root-level `compile_css/2`, `bundle_css/2`, and AST helpers are deprecated compatibility delegates.
- CSS AST printing uses `Vize.CSS.print_ast/2`; the earlier local `generate_css_from_ast` naming was dropped before release.

### Added

- Add `Vize.CSS.parse_ast/2` and `Vize.CSS.parse_ast!/2` — parse CSS into a LightningCSS-backed AST represented as Elixir maps/lists.
- Add `Vize.CSS.print_ast/2` and `Vize.CSS.print_ast!/2` — print CSS from a transformed AST.
- Add `Vize.CSS.walk/2`, `prewalk/2`, `prewalk/3`, `postwalk/2`, `postwalk/3`, and `collect/2` for OXC-style AST traversal.
- Add strict Reach checks to CI: `reach.check --dead-code --smells --strict`.

### Changed

- Update dev/test tooling: Credo, ExDNA, ExDoc, ExSlop, Jason, and Reach.
- Encode/decode CSS AST values across the NIF boundary as BEAM terms instead of JSON strings.

## 0.10.0

- Bump upstream Vize crates 0.43 → 0.76
- Add `:custom_renderer` option to `compile_sfc/2` — treats lowercase non-HTML tags as renderer-native elements instead of Vue components
- Add `:strip_types` option to `compile_sfc/2` — strips TypeScript type annotations via OXC, returning plain JavaScript in a single NIF call
- `compile_sfc/2` result now includes `:macro_artifacts` — compile-time macro artifacts extracted from script blocks (`definePage`, `definePageMeta`, etc.)
- Add `generate_dts/2` — generates `.d.ts` declarations from SFC script analysis
- Fix `:end_` atom → `:end` in loc maps and macro artifacts

## 0.9.0

- Bump upstream Vize crates 0.28 → 0.43
- Rewrite `vapor_split` Rust module for correctness

## 0.8.0

- Add `bundle_css/2` — bundle a CSS file and all its `@import` dependencies into a single stylesheet via LightningCSS's Bundler. Reads files from disk, resolves imports recursively, wraps in `@media`/`@supports`/`@layer` as needed.

## 0.7.0

- Add `css_modules: true` option to `compile_css/2` — enables LightningCSS CSS Modules mode. Class names, IDs, keyframes, and custom identifiers are scoped, result includes `:exports` map of original → hashed names.

## 0.6.0

- Add `vapor_split/1` — compiles a Vue template into a statics/slots split ready for `%Phoenix.LiveView.Rendered{}`. All HTML manipulation (tag tree parsing, element-to-tag mapping, marker injection, splitting) happens in the NIF. Sub-blocks for `v-if` / `v-for` / `v-else` are recursively split.

## 0.5.0

- Precompiled NIF binaries via `RustlerPrecompiled` (aarch64-apple-darwin, x86_64-apple-darwin, aarch64-unknown-linux-gnu, x86_64-unknown-linux-gnu, x86_64-unknown-linux-musl)

## 0.4.0

- Add `compile_css/2` — standalone LightningCSS pipeline with autoprefixing, minification, browser targeting, and Vue scoped styles

## 0.3.0

- Accept `filename` and `scope_id` options in `compile_sfc/2`
- Return `template_hash`, `style_hash`, `script_hash` for HMR change detection
- Encode `key_prop` for `v-for` `:key` attribute in Vapor IR

## 0.2.0

- Expose `is_static` flag and `element_template_map` in Vapor IR
- Encode directive expressions (`v-show`, `v-model`) in Vapor IR

## 0.1.0

- Initial release
- `compile_sfc/1` — compile Vue SFCs to JavaScript + CSS
- `compile_template/1` — standalone template compilation
- `compile_vapor/1` — Vapor mode compilation
- `compile_ssr/1` — SSR compilation
- `vapor_ir/1` — Vapor IR as Elixir maps
- `parse_sfc/1` — parse SFC descriptor
- `lint/2` — lint Vue SFCs
