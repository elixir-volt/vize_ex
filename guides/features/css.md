# CSS

`Vize.CSS` compiles stylesheets with [LightningCSS](https://lightningcss.dev), the same pipeline Vize uses for SFC styles, and adds Sass and parser-backed tools for reading and rewriting CSS.

## Compiling

```elixir
{:ok, result} = Vize.CSS.compile(".a { color: red }", minify: true)
result.code    # ".a{color:red}"
result.errors  # []
```

Options:

- `:minify`: minify the output.
- `:targets`: browsers to support, as major versions, e.g. `%{chrome: 80, safari: 14}`. LightningCSS adds vendor prefixes and lowers newer syntax for them. Vize applies targets while minifying, so they take effect only with `minify: true`.
- `:scoped` and `:scope_id`: scope the styles to a Vue component.
- `:css_modules`: scope class names as CSS Modules.
- `:filename`: used in error messages and for CSS Modules hashes.

```elixir
Vize.CSS.compile(".a { backdrop-filter: blur(2px) }", minify: true, targets: %{safari: 9})
# {:ok, %{code: ".a{-webkit-backdrop-filter:blur(2px);backdrop-filter:blur(2px)}", ...}}
```

### Scoped styles

```elixir
{:ok, result} = Vize.CSS.compile(".foo { color: red }", scoped: true, scope_id: "data-v-abc123")
result.code
# ".foo[data-v-abc123] {\n  color: red;\n}\n"
```

Use the ID from `Vize.SFC.scope_id/2` to match the component's markup; see [Single-file components](sfc.md).

### CSS Modules

```elixir
{:ok, result} = Vize.CSS.compile(".btn { color: red }", css_modules: true, filename: "button.module.css")
result.code     # "._9b48uG_btn {\n  color: red;\n}\n"
result.exports  # %{"btn" => "_9b48uG_btn"}
```

## Bundling

`Vize.CSS.bundle/2` reads an entry file and inlines its `@import`s recursively, wrapping them in the `@media`, `@supports`, and `@layer` rules the imports specify. It takes `:minify`, `:targets`, and `:css_modules`.

```elixir
{:ok, result} = Vize.CSS.bundle("assets/css/app.css", minify: true)
```

## Sass and SCSS

`Vize.CSS.compile_sass/2` compiles Sass with [grass](https://github.com/connorskees/grass), natively:

```elixir
{:ok, %{code: code}} =
  Vize.CSS.compile_sass("$brand: #639; .button { color: $brand; &:hover { opacity: .8 } }")

code
# ".button {\n  color: #639;\n}\n.button:hover {\n  opacity: 0.8;\n}\n"
```

Options:

- `:syntax`: `:scss` (default) or `:sass` for the indented syntax.
- `:filename`: resolves relative imports.
- `:load_paths`: more directories to resolve imports from.
- `:compressed`: emit compressed CSS.

## Reading and rewriting URLs

Find `url()` references with their byte ranges, and rewrite them without re-printing the stylesheet:

```elixir
{:ok, [%Vize.CSS.URL{url: "./logo.svg", range: %Vize.Range{start: 25, end: 35}}]} =
  Vize.CSS.collect_urls(".logo { background: url('./logo.svg') }")

Vize.CSS.rewrite_urls(".logo { background: url('./logo.svg') }", fn
  "./logo.svg" -> {:rewrite, "/assets/logo.svg"}
  _ -> :keep
end)
# {:ok, ".logo { background: url('/assets/logo.svg') }"}
```

`Vize.CSS.select/3` selects parser events by name: `:urls`, `:imports` (with their `supports` and `media` conditions), or `:dependencies` for both.

## Working with the AST

`Vize.CSS.parse_ast/2` returns the LightningCSS AST as Elixir maps and lists. Transform it with `walk/2`, `prewalk/2,3`, `postwalk/2,3`, or `collect/2`, then print it with `print_ast/2`, which takes `:minify` and `:targets`.

```elixir
{:ok, parsed} = Vize.CSS.parse_ast(".foo { background: url('./logo.svg') }")

ast =
  Vize.CSS.postwalk(parsed.ast, fn
    %{"url" => "./logo.svg"} = node -> %{node | "url" => "/assets/logo.svg"}
    node -> node
  end)

{:ok, result} = Vize.CSS.print_ast(ast)
```
