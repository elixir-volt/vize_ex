defmodule Vize do
  @moduledoc """
  Elixir bindings for the [Vize](https://vizejs.dev) Vue.js toolchain.

  Compile, lint, and analyze Vue Single File Components at native speed
  via Rust NIFs. Includes Vapor mode IR for BEAM-native SSR.

      iex> {:ok, result} = Vize.compile_sfc(\"""
      ...> <template><div>{{ msg }}</div></template>
      ...> <script setup>
      ...> const msg = 'hello'
      ...> </script>
      ...> \""")
      iex> result.code =~ "msg"
      true

  ## Vapor IR

  The `vapor_ir/1` function exposes Vue's Vapor mode intermediate
  representation as Elixir maps — enabling BEAM-native SSR without
  executing JavaScript:

      iex> {:ok, ir} = Vize.vapor_ir("<div>{{ msg }}</div>")
      iex> [template] = ir.templates
      iex> template =~ "<div>"
      true
  """

  @type macro_artifact :: %{
          required(:kind) => String.t(),
          required(:name) => String.t(),
          required(:source) => String.t(),
          required(:content) => String.t(),
          required(:start) => non_neg_integer(),
          required(:end) => non_neg_integer(),
          optional(:code) => String.t()
        }

  @type sfc_result :: %{
          code: String.t(),
          source_map: String.t() | nil,
          css: String.t() | nil,
          errors: [map()],
          warnings: [map()],
          template_hash: String.t() | nil,
          style_hash: String.t() | nil,
          script_hash: String.t() | nil,
          has_scoped: boolean(),
          styles: [map()],
          custom_blocks: [map()],
          macro_artifacts: [macro_artifact()]
        }

  @type template_result :: %{
          code: String.t(),
          preamble: String.t(),
          helpers: [String.t()]
        }

  @type vapor_result :: Vize.Vapor.Result.t()

  @type ssr_result :: %{
          code: String.t(),
          preamble: String.t()
        }

  @type ir_result :: %{
          templates: [String.t()],
          components: [String.t()],
          directives: [String.t()],
          block: map(),
          element_template_map: [{non_neg_integer(), non_neg_integer()}]
        }

  @type diagnostic :: %{message: String.t(), name: String.t() | nil}

  # ── SFC Parsing ──

  @doc """
  Parse a Vue Single File Component into its constituent blocks.

  Returns the SFC descriptor with template, script, script_setup, styles,
  and custom_blocks — without compiling.

  ## Examples

      iex> {:ok, descriptor} = Vize.parse_sfc(\"""
      ...> <template><div>hello</div></template>
      ...> <script setup>const x = 1</script>
      ...> <style scoped>.red { color: red }</style>
      ...> \""")
      iex> descriptor.template.content =~ "hello"
      true
      iex> descriptor.script_setup.setup
      true
      iex> hd(descriptor.styles).scoped
      true
  """
  @spec parse_sfc(String.t()) :: {:ok, map()} | {:error, String.t()}
  def parse_sfc(source) do
    Vize.Native.parse_sfc_nif(source)
  end

  @doc """
  Like `parse_sfc/1` but raises on errors.
  """
  @spec parse_sfc!(String.t()) :: map()
  def parse_sfc!(source) do
    case parse_sfc(source) do
      {:ok, descriptor} -> descriptor
      {:error, reason} -> raise "Vize parse error: #{reason}"
    end
  end

  # ── SFC Analysis ──

  @doc """
  Analyze a Vue Single File Component into a semantic Croquis summary.

  ## Options

    * `:mode` — analysis mode: `:full`, `:lint`, `:compile`, or `:declaration`
  """
  @spec analyze_sfc(String.t(), keyword()) :: {:ok, Vize.Croquis.t()} | {:error, Vize.Error.t()}
  def analyze_sfc(source, opts \\ []) do
    mode = opts |> Keyword.get(:mode, :full) |> to_string()

    case Vize.Native.analyze_sfc_nif(source, mode) do
      {:ok, result} -> {:ok, Vize.Croquis.new(result)}
      {:error, reason} -> {:error, error("Vize SFC analysis error", [reason])}
    end
  end

  @doc "Like `analyze_sfc/2` but raises `Vize.Error` on errors."
  @spec analyze_sfc!(String.t(), keyword()) :: Vize.Croquis.t()
  def analyze_sfc!(source, opts \\ []) do
    case analyze_sfc(source, opts) do
      {:ok, croquis} -> croquis
      {:error, error} -> raise error
    end
  end

  # ── SFC Compilation ──

  @doc """
  Compile a Vue Single File Component to JavaScript + CSS.

  Handles `<template>`, `<script>`, `<script setup>`, and `<style>` blocks.

  ## Options

    * `:vapor` — compile in Vapor mode (default: `false`)
    * `:ssr` — compile for server-side rendering (default: `false`)
    * `:filename` — SFC filename for scope ID generation and source maps (e.g. `"App.vue"`)
    * `:scope_id` — explicit scope ID for scoped CSS (default: auto-generated from filename)
    * `:custom_renderer` — treat lowercase non-HTML tags as renderer-native
      elements instead of Vue components (default: `false`)
    * `:strip_types` — strip TypeScript type annotations from the output
      using OXC, returning plain JavaScript (default: `false`)
    * `:source_map` — include a Source Map v3 JSON document for the emitted
      JavaScript when authored script lines can be mapped (default: `false`)

  ## Examples

      iex> {:ok, result} = Vize.compile_sfc(\"""
      ...> <template><button @click="count++">{{ count }}</button></template>
      ...> <script setup>
      ...> import { ref } from 'vue'
      ...> const count = ref(0)
      ...> </script>
      ...> \""")
      iex> result.code =~ "count"
      true
      iex> result.errors
      []
  """
  @spec compile_sfc(String.t(), keyword()) :: {:ok, sfc_result()} | {:error, String.t()}
  def compile_sfc(source, opts \\ []) do
    Vize.Native.compile_sfc_nif(source, %{
      filename: Keyword.get(opts, :filename, ""),
      scope_id: Keyword.get(opts, :scope_id, ""),
      vapor: Keyword.get(opts, :vapor, false),
      ssr: Keyword.get(opts, :ssr, false),
      custom_renderer: Keyword.get(opts, :custom_renderer, false),
      strip_types: Keyword.get(opts, :strip_types, false),
      source_map: Keyword.get(opts, :source_map, false)
    })
  end

  @doc """
  Like `compile_sfc/2` but raises on errors.
  """
  @spec compile_sfc!(String.t(), keyword()) :: sfc_result()
  def compile_sfc!(source, opts \\ []) do
    case compile_sfc(source, opts) do
      {:ok, result} -> result
      {:error, reason} -> raise "Vize compile error: #{reason}"
    end
  end

  # ── Template Compilation ──

  @doc """
  Compile a Vue template string to a render function.

  This compiles just the template (not a full SFC). Useful for
  on-the-fly template compilation.

  ## Options

    * `:mode` — output mode, `"function"` (default) or `"module"`
    * `:ssr` — compile for SSR (default: `false`)

  ## Examples

      iex> {:ok, result} = Vize.compile_template("<div>{{ msg }}</div>")
      iex> result.code =~ "msg"
      true
  """
  @spec compile_template(String.t(), keyword()) ::
          {:ok, template_result()} | {:error, [String.t()]}
  def compile_template(source, opts \\ []) do
    Vize.Native.compile_template_nif(source, %{
      mode: opts |> Keyword.get(:mode, "function") |> to_string(),
      ssr: Keyword.get(opts, :ssr, false)
    })
  end

  @doc """
  Like `compile_template/2` but raises on errors.
  """
  @spec compile_template!(String.t(), keyword()) :: template_result()
  def compile_template!(source, opts \\ []) do
    case compile_template(source, opts) do
      {:ok, result} -> result
      {:error, errors} -> raise "Vize template compile error: #{inspect(errors)}"
    end
  end

  # ── SSR Compilation ──

  @doc """
  Compile a Vue template for server-side rendering.

  Generates JavaScript with `_push()` calls that produce HTML strings.
  The output is meant to be executed in a JS runtime (e.g. QuickBEAM).

  ## Examples

      iex> {:ok, result} = Vize.compile_ssr("<div>{{ msg }}</div>")
      iex> result.code =~ "_push"
      true
  """
  @spec compile_ssr(String.t()) :: {:ok, ssr_result()} | {:error, [String.t()]}
  def compile_ssr(source) do
    Vize.Native.compile_ssr_nif(source)
  end

  @doc """
  Like `compile_ssr/1` but raises on errors.
  """
  @spec compile_ssr!(String.t()) :: ssr_result()
  def compile_ssr!(source) do
    case compile_ssr(source) do
      {:ok, result} -> result
      {:error, errors} -> raise "Vize SSR compile error: #{inspect(errors)}"
    end
  end

  # ── Vapor Mode ──

  @doc """
  Compile a Vue template to Vapor mode JavaScript.

  Vapor mode generates fine-grained reactive code that manipulates
  the DOM directly, without a virtual DOM.

  ## Options

    * `:ssr` — compile for SSR (default: `false`)
    * `:diagnostics` — also report the parser's diagnostics, such as duplicate
      attributes, in the result's `:diagnostics` (default: `false`)
    * `:template_syntax` — `:standard` (default) or `:quirks`

  ## Examples

      iex> {:ok, result} = Vize.compile_vapor("<div>{{ msg }}</div>")
      iex> result.code =~ "template"
      true
      iex> length(result.templates) > 0
      true
  """
  @spec compile_vapor(String.t(), keyword()) :: {:ok, vapor_result()} | {:error, Vize.Error.t()}
  def compile_vapor(source, opts \\ []) do
    nif_opts = %{
      ssr: Keyword.get(opts, :ssr, false),
      diagnostics: Keyword.get(opts, :diagnostics, false),
      template_syntax:
        opts |> Keyword.get(:template_syntax, :standard) |> normalize_template_syntax()
    }

    case Vize.Native.compile_vapor_nif(source, nif_opts) do
      {:ok, result} -> {:ok, Vize.Vapor.Result.new(result)}
      {:error, errors} -> {:error, error("Vize vapor compile error", errors)}
    end
  end

  @doc """
  Like `compile_vapor/2` but raises on errors.
  """
  @spec compile_vapor!(String.t(), keyword()) :: vapor_result()
  def compile_vapor!(source, opts \\ []) do
    case compile_vapor(source, opts) do
      {:ok, result} -> result
      {:error, error} -> raise error
    end
  end

  # ── Vapor IR ──

  @doc """
  Get the Vapor mode intermediate representation as Elixir maps.

  This is the key function for BEAM-native SSR. Instead of generating
  JavaScript, it returns the structured IR that describes how to render
  the template — enabling a pure Elixir renderer.

  The IR contains:

    * `:templates` — static HTML template strings
    * `:components` — component names used in the template
    * `:directives` — directive names used in the template
    * `:block` — the root block with operations, effects, and returns
    * `:element_template_map` — list of `{element_id, template_index}` tuples

  Expressions are either plain strings (dynamic) or `{:static, value}`
  tuples (compile-time constants).

  Each operation has a `:kind` field indicating its type:

    * `:set_prop` — set an attribute/property on an element
    * `:set_text` — set text content (interpolation)
    * `:set_event` — bind an event handler
    * `:set_html` — set innerHTML (v-html)
    * `:if_node` — v-if/v-else-if/v-else chain
    * `:for_node` — v-for loop
    * `:create_component` — child component
    * `:child_ref` / `:next_ref` — DOM traversal helpers

  ## Examples

      iex> {:ok, ir} = Vize.vapor_ir("<div :class=\\"cls\\">{{ msg }}</div>")
      iex> [_template] = ir.templates
      iex> ir.block.effects |> List.flatten() |> Enum.any?(&match?(%{kind: :set_text}, &1))
      true
  """
  @spec vapor_ir(String.t()) :: {:ok, ir_result()} | {:error, [String.t()]}
  def vapor_ir(source) do
    Vize.Native.vapor_ir_nif(source)
  end

  @doc """
  Like `vapor_ir/1` but raises on errors.
  """
  @spec vapor_ir!(String.t()) :: ir_result()
  def vapor_ir!(source) do
    case vapor_ir(source) do
      {:ok, ir} -> ir
      {:error, errors} -> raise "Vize vapor IR error: #{inspect(errors)}"
    end
  end

  @doc """
  Split a Vue template into static HTML and the dynamic slots between them,
  the shape of `%Phoenix.LiveView.Rendered{}`.

  The template is lowered to Vize's L2 semantic IR, which its SSR compiler
  also uses, and printed: elements, static attributes, and text become HTML,
  and everything that depends on data becomes a slot. Returns `{:ok, split}`:

    * `:statics` — static HTML strings, one more than there are slots
    * `:slots` — slots in document order. Each has a `:kind` and a
      `:position`, `{line, column}` in the template. Expressions are
      JavaScript source strings.
      * `:text` — an escaped interpolation (`:value`)
      * `:html` — unescaped HTML from `v-html` (`:value`)
      * `:attr` — one attribute, rendered whole with its leading space, so a
        renderer can leave it out, as Vue does for `null` and a false boolean
        attribute. Has `:name` (or `:name_value` for `:[name]`), `:value`,
        `:static` for a static attribute of the same name such as `class="a"`
        beside `:class="b"`, and `:show` for the `v-show` a `style` combines with
      * `:spread` — the attributes of an object, from `v-bind="attrs"` (`:value`)
      * `:model` — what a `v-model` renders on an `<input>` or `<textarea>`:
        `:value`, the element's `:tag`, and its static `:type` and `:static_value`
      * `:if` — `:branches`, each a `:condition` (nil for `v-else`) and a `:block`
      * `:for` — `:source`, `:value`, `:key`, `:index`, the repeated element's
        `:key_prop`, and its `:block`
      * `:component` — `:name`, `:props`, `:events`, and `:slots`: the content
        passed to each of its slots, with its `:name`, `:params` pattern such
        as `{ item }`, and `:block`
      * `:slot` — a `<slot>` outlet: `:name`, `:props`, and a `:fallback` block
      * `:root_attrs` — with `root_attrs: true`, see below
    * `:bindings` — events and `v-model`s, left for the caller to render: each
      has a `:kind` (`:on` or `:model`), `:name`, `:modifiers`, `:value`, and
      `:at`, a `{static_index, offset}` pair where the element's start tag
      ends, so attributes can be inserted there
    * `:diagnostics` — warnings, as `Vize.Diagnostic` structs

  A block, such as a `v-if` branch, has its own `:statics`, `:slots`, and
  `:bindings`. A prop or attribute in `:props` has `:name` (or `:name_value`),
  and `:static` or `:value`; one with neither name is a `v-bind` object.

  Returns `{:error, %Vize.Error{}}` when the template has errors, such as an
  expression that doesn't parse.

  ## Options

    * `:root_attrs` — for a component's template: when it has a single root
      element, return all of that element's attributes, static ones included,
      as one `:root_attrs` slot, so a caller can merge the
      [fallthrough attributes](https://vuejs.org/guide/components/attrs.html)
      a parent passes (default: `false`)

  ## Examples

      iex> {:ok, split} = Vize.split_template(~s(<p :class="kind">Hi {{ name }}</p>))
      iex> split.statics
      ["<p", ">Hi ", "</p>"]
      iex> Enum.map(split.slots, & &1.kind)
      [:attr, :text]
  """
  @spec split_template(String.t(), keyword()) :: {:ok, map()} | {:error, Vize.Error.t()}
  def split_template(source, opts \\ []) do
    nif_opts = %{root_attrs: Keyword.get(opts, :root_attrs, false)}

    case Vize.Native.split_template_nif(source, nif_opts) do
      {:ok, split} ->
        {:ok, %{split | diagnostics: Enum.map(split.diagnostics, &Vize.Diagnostic.new/1)}}

      {:error, diagnostics} ->
        {:error, error("Vue template has errors", diagnostics)}
    end
  end

  @doc "Like `split_template/2` but raises `Vize.Error` on errors."
  @spec split_template!(String.t(), keyword()) :: map()
  def split_template!(source, opts \\ []) do
    case split_template(source, opts) do
      {:ok, split} -> split
      {:error, error} -> raise error
    end
  end

  # ── Linting ──

  @doc """
  Lint a Vue SFC source string.

  Returns a list of diagnostics with `:message` and optionally `:name`
  (the rule name).

  ## Examples

      iex> {:ok, diagnostics} = Vize.lint("<template><img></template>", "test.vue")
      iex> is_list(diagnostics)
      true
  """
  @spec lint(String.t(), String.t()) :: {:ok, [diagnostic()]}
  def lint(source, filename \\ "component.vue") do
    Vize.Native.lint_nif(source, filename)
  end

  # ── CSS Compilation ──

  @type css_result :: Vize.CSS.css_result()

  @deprecated "Use Vize.CSS.compile/2 instead"
  defdelegate compile_css(source, opts \\ []), to: Vize.CSS, as: :compile

  @deprecated "Use Vize.CSS.compile!/2 instead"
  defdelegate compile_css!(source, opts \\ []), to: Vize.CSS, as: :compile!

  @deprecated "Use Vize.CSS.bundle/2 instead"
  defdelegate bundle_css(entry_path, opts \\ []), to: Vize.CSS, as: :bundle

  @deprecated "Use Vize.CSS.bundle!/2 instead"
  defdelegate bundle_css!(entry_path, opts \\ []), to: Vize.CSS, as: :bundle!

  @deprecated "Use Vize.CSS.parse_ast/2 instead"
  defdelegate parse_css_ast(source, opts \\ []), to: Vize.CSS, as: :parse_ast

  @deprecated "Use Vize.CSS.parse_ast!/2 instead"
  defdelegate parse_css_ast!(source, opts \\ []), to: Vize.CSS, as: :parse_ast!

  @deprecated "Use Vize.CSS.print_ast/2 instead"
  defdelegate print_css_ast(ast, opts \\ []), to: Vize.CSS, as: :print_ast

  @deprecated "Use Vize.CSS.print_ast!/2 instead"
  defdelegate print_css_ast!(ast, opts \\ []), to: Vize.CSS, as: :print_ast!

  # ── Declaration .d.ts Generation ──

  @type dts_result :: %{dts: String.t()}

  @doc """
  Generate a TypeScript declaration file (`.d.ts`) from a Vue SFC.

  Analyzes the SFC's script blocks and produces a lightweight type
  surface for component consumers — including prop types, emit signatures,
  exposed bindings, and slot definitions.

  ## Options

    * `:filename` — SFC filename for diagnostics (default: `"component.vue"`)

  ## Examples

      iex> {:ok, result} = Vize.generate_dts("<script setup>const msg = 1</script>")
      iex> is_binary(result.dts)
      true
  """
  @spec generate_dts(String.t(), keyword()) :: {:ok, dts_result()} | {:error, String.t()}
  def generate_dts(source, opts \\ []) do
    filename = Keyword.get(opts, :filename, "")
    Vize.Native.generate_dts_nif(source, filename)
  end

  @doc "Like `generate_dts/2` but raises on errors."
  @spec generate_dts!(String.t(), keyword()) :: dts_result()
  def generate_dts!(source, opts \\ []) do
    case generate_dts(source, opts) do
      {:ok, result} -> result
      {:error, reason} -> raise "Vize declaration generation error: #{reason}"
    end
  end

  defp normalize_template_syntax(:standard), do: "standard"
  defp normalize_template_syntax(:quirks), do: "quirks"
  defp normalize_template_syntax("standard"), do: "standard"
  defp normalize_template_syntax("quirks"), do: "quirks"

  defp error(message, errors), do: Vize.Error.new(message, errors)
end
