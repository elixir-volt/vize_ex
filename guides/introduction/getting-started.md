# Getting Started

## Installation

Add Vize to your dependencies:

```elixir
def deps do
  [{:vize, "~> 0.15"}]
end
```

Vize is a Rust NIF. `mix deps.get` downloads a precompiled build for your platform:

- macOS on Apple Silicon and Intel
- Linux on x86-64 and ARM64 (glibc), and x86-64 (musl)
- Windows on x86-64 (MSVC)

### Building from source

Set `VIZE_EX_BUILD=1` to compile the NIF instead of downloading it. This needs Rust 1.95 or later; `rustup` is the easiest way to install it.

```bash
VIZE_EX_BUILD=1 mix deps.compile vize
```

## First steps

Every function returns `{:ok, result}` or `{:error, reason}`, and most have a `!` variant that raises instead.

Compile a component:

```elixir
source = """
<template><p :class="cls">{{ msg }}</p></template>
<script setup>
const cls = "greeting"
const msg = "Hello"
</script>
<style scoped>p { color: red }</style>
"""

{:ok, result} = Vize.compile_sfc(source, filename: "Greeting.vue")
result.code    # JavaScript module
result.css     # CSS scoped to this component
result.errors  # []
```

Read it without compiling:

```elixir
{:ok, descriptor} = Vize.parse_sfc(source)
descriptor.template.content
descriptor.script_setup.content
hd(descriptor.styles).scoped  # true
```

Compile CSS on its own:

```elixir
{:ok, result} = Vize.CSS.compile(".a { color: red }", minify: true)
result.code  # ".a{color:red}"
```

## Next

- [Single-file components](sfc.md): compile options, hashes, scope IDs, assets, and `.d.ts`
- [Analysis and linting](analysis-and-linting.md)
- [Vapor mode](vapor.md) and the Vapor IR
- [Templates and SSR](templates-and-ssr.md)
- [CSS](css.md)
- [API cheatsheet](api.cheatmd)
