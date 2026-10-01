# Templates and SSR

## Standalone templates

`Vize.compile_template/2` compiles a template, without an SFC around it, to a render function:

```elixir
{:ok, result} = Vize.compile_template(~s(<div v-if="show">{{ msg }}</div>))

result.code      # render function
result.preamble  # imports
result.helpers   # ["openBlock", "createBlock", "Fragment", "createCommentVNode", ...]
```

Options:

- `:mode`: `"function"` (default) for a plain function, or `"module"` for an ES module.
- `:ssr`: compile for server-side rendering.

## Server-side rendering

`Vize.compile_ssr/1` compiles a template to code that writes HTML with `_push()`. Run it in a JavaScript runtime such as [QuickBEAM](https://github.com/elixir-volt/quickbeam).

```elixir
{:ok, result} = Vize.compile_ssr("<div>{{ msg }}</div>")

result.code
# function ssrRender(_ctx, _push, _parent, _attrs) {
#   _push(`<div${_ssrRenderAttrs(_attrs)}>${_ssrInterpolate(_ctx.msg)}</div>`)
# }

result.preamble
# import { ssrInterpolate as _ssrInterpolate, ssrRenderAttrs as _ssrRenderAttrs } from "@vue/server-renderer"
```
