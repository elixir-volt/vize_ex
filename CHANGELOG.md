# Changelog

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
