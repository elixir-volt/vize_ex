# Single-File Components

A [single-file component](https://vuejs.org/guide/scaling-up/sfc.html) is a `.vue` file with a template, scripts, and styles. Vue's [SFC syntax specification](https://vuejs.org/api/sfc-spec.html) describes the blocks.

## Parsing

`Vize.parse_sfc/1` returns the descriptor: the template, `<script>`, `<script setup>`, styles, and custom blocks, each with its content, `lang`, attributes, and location.

```elixir
{:ok, descriptor} = Vize.parse_sfc(source)

descriptor.script_setup
# %{content: "...", lang: "ts", setup: true, attrs: %{...}, loc: %{...}, ...}
```

A block's `loc` gives both spans in the source, as byte offsets:

- `start`/`end`: the block's content
- `tag_start`/`tag_end`: the whole block, including its opening and closing tags

It also has one-based `start_line`, `start_column`, `end_line`, and `end_column` for the content.

```elixir
%{tag_start: s, tag_end: e} = descriptor.script_setup.loc
binary_part(source, s, e - s)
# "<script setup lang=\"ts\">...</script>"
```

## Compiling

`Vize.compile_sfc/2` compiles a component to a JavaScript module and its CSS.

```elixir
{:ok, result} = Vize.compile_sfc(source, filename: "App.vue")

result.code      # JavaScript
result.css       # compiled styles
result.errors    # compile errors
result.warnings
```

Options:

- `:filename`: used for scope IDs, source maps, and error messages. Pass it so scoped-style attributes stay stable.
- `:scope_id`: use this scope ID instead of deriving one from the filename.
- `:vapor`: compile the template in Vapor mode.
- `:ssr`: compile for server-side rendering.
- `:strip_types`: strip TypeScript from the output.
- `:source_map`: include a Source Map v3 JSON document for the script.
- `:custom_renderer`: treat lowercase tags that aren't HTML as renderer-native.

### Content hashes

The result carries a hash for each part, so a bundler can tell which part changed and decide between a template hot update and a reload:

```elixir
result.template_hash  # "c85bb72e417ef37f"
result.style_hash     # "b28394c43675a763"
result.script_hash    # nil when there is no script
```

### Scope IDs

`Vize.SFC.scope_id/2` computes the ID the way Vize's bundler integrations do. Pass the same ID to `compile_sfc/2` and to `Vize.CSS.compile/2` to scope styles consistently.

```elixir
Vize.SFC.scope_id("src/App.vue", root: "/app")
# "7a7a37b1"
```

Options: `:root` normalizes the filename, `:production` switches to the production hashing strategy, and `:source` adds the source to it.

## Bundler helpers

`Vize.SFC.collect_template_assets/2` finds static asset references in the template, such as `img[src]`, `video[poster]`, and SVG `use` references:

```elixir
{:ok, assets} = Vize.SFC.collect_template_assets(source, filename: "App.vue")
# [%Vize.SFC.Asset{url: "./logo.png", binding: "_imports_0"}]
```

`Vize.SFC.rewrite_asset_references/2` then rewrites those literals in the compiled JavaScript to the given import bindings.

`Vize.SFC.external_sources/1` lists blocks loaded through a `src` attribute, such as `<style src="./theme.css">`.

## Type declarations

`Vize.generate_dts/2` produces a `.d.ts` for a component's props, emits, and slots:

```elixir
{:ok, %{dts: dts}} = Vize.generate_dts(source, filename: "App.vue")
# export type Props = { msg: string };
# ...
```
