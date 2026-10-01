# Vize

[![Hex.pm](https://img.shields.io/hexpm/v/vize.svg)](https://hex.pm/packages/vize) [![Documentation](https://img.shields.io/badge/documentation-gray)](https://hexdocs.pm/vize)

Elixir bindings for [Vize](https://vizejs.dev), a Vue.js toolchain written in Rust. Parse, analyze, lint, and compile Vue single-file components from Elixir, including Vapor mode and its intermediate representation, plus a LightningCSS-based CSS pipeline.

```elixir
{:ok, result} =
  Vize.compile_sfc("""
  <template><button @click="count++">{{ count }}</button></template>
  <script setup>
  import { ref } from "vue"
  const count = ref(0)
  </script>
  <style scoped>button { color: blue }</style>
  """, filename: "Counter.vue")

result.code  # JavaScript module
result.css   # scoped CSS
```

## Installation

```elixir
def deps do
  [{:vize, "~> 0.15"}]
end
```

Precompiled NIFs are downloaded for macOS, Linux (glibc and musl), and Windows, so no Rust toolchain is needed. To build from source, set `VIZE_EX_BUILD=1` and install Rust 1.95 or later.

## What it covers

- **SFCs**: parse descriptors with block locations, compile to DOM, Vapor, or SSR JavaScript with scoped CSS and content hashes, generate `.d.ts` declarations, and collect template assets and scope IDs for bundlers. See [Single-file components](https://hexdocs.pm/vize/sfc.html).
- **Analysis and linting**: bindings, props, emits, component usages, and template expressions from Vize's semantic analysis, plus lint diagnostics. See [Analysis and linting](https://hexdocs.pm/vize/analysis-and-linting.html).
- **Vapor mode**: compile to Vapor JavaScript, or get the Vapor IR as Elixir maps for rendering on the BEAM. See [Vapor mode](https://hexdocs.pm/vize/vapor.html).
- **Templates and SSR**: compile standalone templates to render functions, or to SSR code that `_push()`es HTML. See [Templates and SSR](https://hexdocs.pm/vize/templates-and-ssr.html).
- **CSS**: compile, scope, minify, autoprefix, and bundle CSS with LightningCSS, compile Sass and SCSS, and rewrite or transform stylesheets through a parser-backed AST. See [CSS](https://hexdocs.pm/vize/css.html).

[Volt](https://github.com/elixir-volt/volt) uses Vize to build Vue apps, and [PhoenixVapor](https://github.com/elixir-volt/phoenix_vapor) uses it to render Vue templates as LiveView.

## Documentation

Guides, a cheatsheet, and the API reference are on [HexDocs](https://hexdocs.pm/vize).

## Part of Elixir Volt

vize compiles and analyzes Vue single-file components from Elixir, including Vapor-mode IR for BEAM-native rendering.

It is part of a frontend stack that runs inside the BEAM — builds, JS
runtimes, icons, and Vue-to-LiveView compilation as supervised parts of the
application instead of external toolchain processes. See the
[Elixir Volt](https://github.com/elixir-volt) organization for the rest, and
[Building Blocks for the Future Web](https://github.com/elixir-vibe/building-blocks)
for the thesis, architecture, and roadmap that tie them together.

## License

[MIT](./LICENSE)
