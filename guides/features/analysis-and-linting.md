# Analysis and Linting

## Semantic analysis

`Vize.analyze_sfc/2` returns a `%Vize.Croquis{}` summary from Vize's semantic analysis of the script and template:

```elixir
{:ok, croquis} = Vize.analyze_sfc(source)

croquis.props
# [%{name: "msg", required: true}]

croquis.bindings
# [%{name: "props", kind: "SetupReactiveConst"}, %{name: "msg", kind: "Props"}, ...]

Enum.map(croquis.template_expressions, & &1.source)
# ["cls", "msg"]
```

Fields:

- `props`, `emits`, `models`: declarations from `defineProps`, `defineEmits`, and `defineModel`. All forms are read: arrays, objects, and TypeScript type arguments.
- `bindings`: every top-level binding with its kind, such as `SetupRef`, `Props`, or `LiteralConst`.
- `used_components`, `used_directives`, `component_usages`: what the template uses, including the props and events passed to each component.
- `template_expressions`: every expression in the template, with its source and range.
- `undefined_refs`: names the template reads that nothing defines.
- `stats`: counts for each of the above.

The `:mode` option (`:full`, `:lint`, `:compile`, or `:declaration`) selects how much analysis runs.

## Linting

`Vize.lint/2` runs Vize's built-in rules:

```elixir
{:ok, diagnostics} = Vize.lint(~s(<template><img src="a.png"></template>), "App.vue")
# [%{message: "<img> elements must have an alt attribute", name: "a11y/alt-text"}]
```
