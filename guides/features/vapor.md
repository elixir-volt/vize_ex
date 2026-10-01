# Vapor Mode

Vapor is Vue's compilation mode without a virtual DOM: a template compiles to code that creates and updates DOM nodes directly.

## Compiling

```elixir
{:ok, result} = Vize.compile_vapor("<div>{{ msg }}</div>")
result.code       # Vapor JavaScript
result.templates  # static HTML templates
```

Options:

- `:ssr`: compile for server-side rendering.
- `:diagnostics`: also report the parser's diagnostics, such as duplicate attributes, which are otherwise not reported.
- `:template_syntax`: `:standard` (default) or `:quirks`.

```elixir
{:ok, %Vize.Vapor.Result{diagnostics: diagnostics}} =
  Vize.compile_vapor(~s(<div id="a" id="b">x</div>), diagnostics: true)

[%Vize.Diagnostic{code: "DuplicateAttribute", recoverable?: true}] = diagnostics
```

## The Vapor IR

`Vize.vapor_ir/1` returns the intermediate representation the Vapor compiler builds, as Elixir maps. That lets you render Vue templates on the BEAM without a JavaScript runtime.

```elixir
{:ok, ir} = Vize.vapor_ir(~s(<div :class="cls">{{ msg }}</div>))

ir.templates             # ["<div> </div>"]
ir.element_template_map  # [{0, 0}], element ID to template index
ir.block.effects
# [
#   [%{kind: :set_prop, element: 0, value: %{key: {:static_, "class"}, values: ["cls"], ...}, ...}],
#   [%{kind: :set_text, element: 0, values: ["msg"], ...}]
# ]
```

Every node has a `:kind`:

| Kind | Vue feature |
| --- | --- |
| `:set_text` | `{{ expr }}` |
| `:set_prop` | `:attr="expr"` |
| `:set_html` | `v-html` |
| `:set_dynamic_props` | `v-bind="obj"` |
| `:set_event` | `@event="handler"` |
| `:if_node` | `v-if` / `v-else-if` / `v-else` |
| `:for_node` | `v-for` |
| `:create_component` | `<Component />` |
| `:directive` | `v-show`, `v-model`, custom directives |

Expressions are strings; static values are `{:static_, "value"}` tuples.

## Splitting templates for LiveView

`Vize.vapor_split/1` turns a template into static HTML strings and the dynamic slots between them, shaped for `%Phoenix.LiveView.Rendered{}`. `v-if` and `v-for` blocks are split recursively.

```elixir
{:ok, split} = Vize.vapor_split(~s(<div :class="cls"><p>{{ msg }}</p></div>))

split.statics  # ["<div class=\"", "\"><p>", "</p></div>"]
split.slots    # [%{kind: :set_prop, values: ["cls"]}, %{kind: :set_text, values: ["msg"]}]
```

The current split also applies LiveView conventions: event handlers become `phx-*` attributes, and `v-model` adds `phx-change`. Those conventions are moving to [PhoenixVapor](https://github.com/elixir-volt/phoenix_vapor), which renders Vue templates as LiveView. A later release will report events as data on the split instead.
