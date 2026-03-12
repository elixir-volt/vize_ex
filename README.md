# Vize

Elixir bindings for the [Vize](https://vizejs.dev) Vue.js toolchain via Rust NIFs.

Compile, lint, and analyze Vue Single File Components at native speed —
including Vapor mode IR for BEAM-native SSR.

## Features

- **Compile** Vue SFCs to JavaScript + CSS (DOM, Vapor, SSR modes)
- **Template** compilation — standalone template → render function
- **Vapor IR** — get the intermediate representation as Elixir maps for BEAM-native rendering
- **SSR** — server-side rendering compilation with `_push()` codegen
- **Lint** Vue SFCs with built-in rules
- **Content hashes** — template, script, and style hashes for HMR change detection
- **CSS compilation** — standalone LightningCSS pipeline with autoprefixing, minification, and Vue scoped styles

## Installation

```elixir
def deps do
  [
    {:vize, "~> 0.6.0"}
  ]
end
```

Requires a Rust toolchain (`rustup` recommended). The NIF compiles automatically on `mix compile`.

## Usage

### Compile SFC

```elixir
{:ok, result} = Vize.compile_sfc("""
<template>
  <button @click="count++">{{ count }}</button>
</template>

<script setup>
import { ref } from 'vue'
const count = ref(0)
</script>

<style scoped>
button { color: blue; }
</style>
""")

result.code     # Generated JavaScript
result.css      # Compiled CSS
result.errors   # []
```

Pass a filename for stable scoped CSS `data-v-xxxx` attributes and content hashes:

```elixir
{:ok, result} = Vize.compile_sfc(source, filename: "App.vue")

result.template_hash  # "de5ddf78a0f8d31a"
result.style_hash     # "3efafd39ec9747f9"
result.script_hash    # "1a8dae0fef50c189"
```

### Vapor Mode

```elixir
{:ok, result} = Vize.compile_vapor("<div>{{ msg }}</div>")
result.code       # Vapor JS (no virtual DOM)
result.templates  # Static HTML templates
```

### Vapor IR (for BEAM-native SSR)

```elixir
{:ok, ir} = Vize.vapor_ir("<div :class=\"cls\">{{ msg }}</div>")

ir.templates             # ["<div> </div>"]
ir.element_template_map  # [{0, 0}]  — element ID → template index
ir.block                 # %{operations: [...], effects: [...], returns: [...]}
```

The IR exposes every Vue construct as Elixir maps with `:kind` atoms:

| Kind | Vue Feature |
|------|-------------|
| `:set_text` | `{{ expr }}` |
| `:set_prop` | `:attr="expr"` |
| `:set_html` | `v-html` |
| `:set_dynamic_props` | `v-bind="obj"` |
| `:set_event` | `@event="handler"` |
| `:if_node` | `v-if` / `v-else-if` / `v-else` |
| `:for_node` | `v-for` |
| `:create_component` | `<Component />` |
| `:directive` | `v-show`, `v-model`, custom |

Static expressions are tagged as `{:static, "value"}` tuples,
dynamic expressions are plain strings.

### Vapor Split (for Phoenix LiveView)

```elixir
{:ok, split} = Vize.vapor_split("<div :class=\"cls\"><p>{{ msg }}</p></div>")

split.statics  # ["<div class=\"", "\"><p>", "</p></div>"]
split.slots    # [%{kind: :set_prop, values: ["cls"]}, %{kind: :set_text, values: ["msg"]}]
```

Produces a statics/slots split ready for `%Phoenix.LiveView.Rendered{}`.
All HTML manipulation (tag tree parsing, marker injection, splitting) happens
in the NIF. Sub-blocks for `v-if` / `v-for` are recursively split.

Used by [PhoenixVapor](https://github.com/elixir-volt/phoenix_vapor) to render
Vue templates as native LiveView output.

### SSR Compilation

```elixir
{:ok, result} = Vize.compile_ssr("<div>{{ msg }}</div>")
result.code      # JS with _push() calls
result.preamble  # Import statements
```

### Template Compilation

```elixir
{:ok, result} = Vize.compile_template("<div v-if=\"show\">{{ msg }}</div>")
result.code     # Render function
result.helpers  # ["createElementVNode", "toDisplayString", ...]
```

### Lint

```elixir
{:ok, diagnostics} = Vize.lint("<template><img></template>", "App.vue")
```

### Parse SFC

```elixir
{:ok, descriptor} = Vize.parse_sfc(source)
descriptor.template      # %{content: "...", lang: nil, ...}
descriptor.script_setup  # %{content: "...", setup: true, ...}
descriptor.styles        # [%{content: "...", scoped: true, ...}]
```

### CSS Compilation

Standalone CSS compilation via LightningCSS — parse, autoprefix, and minify CSS
independently of SFC compilation:

```elixir
{:ok, result} = Vize.compile_css(".foo { color: red; user-select: none }", minify: true)
result.code
# ".foo{color:red;-webkit-user-select:none;user-select:none}"
```

With Vue scoped styles:

```elixir
{:ok, result} = Vize.compile_css(".foo { color: red }", scoped: true, scope_id: "data-v-abc123")
result.code
# ".foo[data-v-abc123] { color: red }"
```

Browser targeting:

```elixir
{:ok, result} = Vize.compile_css(css, targets: %{chrome: 80, firefox: 78, safari: 14})
```

## License

[MIT](./LICENSE)
