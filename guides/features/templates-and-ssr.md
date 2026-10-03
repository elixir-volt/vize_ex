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

`Vize.compile_ssr/1` compiles a template for Vue's [server-side rendering](https://vuejs.org/guide/scaling-up/ssr.html), to code that writes HTML with `_push()`. Run it in a JavaScript runtime such as [QuickBEAM](https://github.com/elixir-volt/quickbeam).

```elixir
{:ok, result} = Vize.compile_ssr("<div>{{ msg }}</div>")

result.code
# function ssrRender(_ctx, _push, _parent, _attrs) {
#   _push(`<div${_ssrRenderAttrs(_attrs)}>${_ssrInterpolate(_ctx.msg)}</div>`)
# }

result.preamble
# import { ssrInterpolate as _ssrInterpolate, ssrRenderAttrs as _ssrRenderAttrs } from "@vue/server-renderer"
```

## Splitting templates

`Vize.split_template/2` turns a template into static HTML and the dynamic slots between them, the shape of `%Phoenix.LiveView.Rendered{}`, so a server can render it without JavaScript. [PhoenixVapor](https://github.com/elixir-volt/phoenix_vapor) uses it to render Vue templates in Phoenix LiveView.

It works from Vize's L2 semantic IR, which Vize's SSR compiler also builds on: elements, static attributes and text become HTML, and everything that depends on data becomes a slot. Expressions are JavaScript source strings, and every slot has a `:position`, `{line, column}` in the template.

```elixir
{:ok, split} = Vize.split_template(~s(<p :class="kind">Hi {{ name }}</p>))

split.statics  # ["<p", ">Hi ", "</p>"]
split.slots    # [%{kind: :attr, name: "class", value: "kind", static: nil, ...}, %{kind: :text, value: "name", ...}]
```

An `:attr` slot stands for the whole attribute with its leading space, so a renderer can leave it out, as Vue does for `null` and a false [boolean attribute](https://vuejs.org/guide/essentials/template-syntax.html#boolean-attributes). A static attribute of the same name, such as `class="a"` beside `:class="b"`, arrives as its `:static`.

`v-if` and `v-for` slots carry a block each: its own `:statics`, `:slots` and `:bindings`. A component slot has its `:props`, `:events`, and the content passed to each of its [slots](https://vuejs.org/guide/components/slots.html); a `<slot>` outlet is a slot of its own:

```elixir
{:ok, split} = Vize.split_template(~s(<Card title="Hi"><p>Body</p><template #footer="{ ok }">{{ ok }}</template></Card>))

[%{kind: :component, name: "Card", props: [%{name: "title", static: "Hi"}], slots: slots}] = split.slots
slots  # [%{name: "footer", params: "{ ok }", block: %{statics: ["", ""], ...}}, %{name: "default", block: ...}]
```

A component's own template can be split with `root_attrs: true`. When it has a single root element, all of that element's attributes, static ones included, come back as one `:root_attrs` slot, so the caller can merge in the [fallthrough attributes](https://vuejs.org/guide/components/attrs.html) the parent passed:

```elixir
{:ok, split} = Vize.split_template(~s(<button class="btn" type="button"><slot /></button>), root_attrs: true)

split.statics  # ["<button", ">", "</button>"]
split.slots    # [%{kind: :root_attrs, props: [%{name: "class", static: "btn"}, %{name: "type", static: "button"}]}, %{kind: :slot, ...}]
```

Events and `v-model`s are not rendered. They are reported as bindings, with the position where their element's start tag ends, so a caller can add whatever attributes its runtime needs:

```elixir
{:ok, split} = Vize.split_template(~s(<button @click="save">{{ label }}</button>))

split.statics   # ["<button>", "</button>"]
split.bindings  # [%{kind: :on, name: "click", value: "save", modifiers: [], at: {0, 7}, ...}]
```

`at: {0, 7}` is byte 7 of the first static, just before the `>`. PhoenixVapor adds `phx-click="save"` there.

A template with errors, such as an expression that doesn't parse, returns `{:error, %Vize.Error{}}` with a `Vize.Diagnostic` for each, and warnings come back in the split's `:diagnostics`. `Vize.Diagnostic.to_code_diagnostic/2` converts one to Elixir's `t:Code.diagnostic/1` shape for a file, offset to where the template starts in it.
