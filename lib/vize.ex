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

  @type sfc_result :: %{
          code: String.t(),
          css: String.t() | nil,
          errors: [map()],
          warnings: [map()]
        }

  @type template_result :: %{
          code: String.t(),
          preamble: String.t(),
          helpers: [String.t()]
        }

  @type vapor_result :: %{
          code: String.t(),
          templates: [String.t()]
        }

  @type ssr_result :: %{
          code: String.t(),
          preamble: String.t()
        }

  @type ir_result :: %{
          templates: [String.t()],
          components: [String.t()],
          directives: [String.t()],
          block: map()
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

  # ── SFC Compilation ──

  @doc """
  Compile a Vue Single File Component to JavaScript + CSS.

  Handles `<template>`, `<script>`, `<script setup>`, and `<style>` blocks.

  ## Options

    * `:vapor` — compile in Vapor mode (default: `false`)
    * `:ssr` — compile for server-side rendering (default: `false`)

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
    vapor = Keyword.get(opts, :vapor, false)
    ssr = Keyword.get(opts, :ssr, false)
    Vize.Native.compile_sfc_nif(source, vapor, ssr)
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
    mode = opts |> Keyword.get(:mode, "function") |> to_string()
    ssr = Keyword.get(opts, :ssr, false)
    Vize.Native.compile_template_nif(source, mode, ssr)
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

  ## Examples

      iex> {:ok, result} = Vize.compile_vapor("<div>{{ msg }}</div>")
      iex> result.code =~ "template"
      true
      iex> length(result.templates) > 0
      true
  """
  @spec compile_vapor(String.t(), keyword()) :: {:ok, vapor_result()} | {:error, [String.t()]}
  def compile_vapor(source, opts \\ []) do
    ssr = Keyword.get(opts, :ssr, false)
    Vize.Native.compile_vapor_nif(source, ssr)
  end

  @doc """
  Like `compile_vapor/2` but raises on errors.
  """
  @spec compile_vapor!(String.t(), keyword()) :: vapor_result()
  def compile_vapor!(source, opts \\ []) do
    case compile_vapor(source, opts) do
      {:ok, result} -> result
      {:error, errors} -> raise "Vize vapor compile error: #{inspect(errors)}"
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
end
