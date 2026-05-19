defmodule Vize.CSS do
  @moduledoc """
  CSS parsing, printing, and AST traversal helpers.

  The AST is produced by LightningCSS and represented as Elixir maps, lists,
  strings, numbers, booleans, and `nil`. Use `parse_ast/2` to parse CSS,
  transform the returned AST with `prewalk/2` or `postwalk/2`, and then use
  `print_ast/2` to serialize it back to CSS.
  """

  @type css_result :: %{
          optional(:exports) => %{optional(String.t()) => String.t()} | nil,
          code: String.t(),
          css_vars: [String.t()],
          errors: [String.t()],
          warnings: [String.t()]
        }

  @type ast :: map() | list() | String.t() | number() | boolean() | nil

  @type ast_result :: %{
          ast: map() | nil,
          errors: [String.t()],
          warnings: [String.t()]
        }

  @doc """
  Parse CSS into a LightningCSS-backed AST represented as Elixir maps and lists.

  ## Options

    * `:filename` — filename for parser locations and error reporting
    * `:css_modules` — enable CSS Modules parsing (default: `false`)
    * `:custom_media` — enable custom media parsing (default: `false`)

  ## Examples

      iex> {:ok, result} = Vize.CSS.parse_ast(".foo { background: url('./logo.svg') }")
      iex> is_map(result.ast)
      true
      iex> result.errors
      []
  """
  @spec parse_ast(String.t(), keyword()) :: {:ok, ast_result()}
  def parse_ast(source, opts \\ []) do
    filename = Keyword.get(opts, :filename, "")
    css_modules = Keyword.get(opts, :css_modules, false)
    custom_media = Keyword.get(opts, :custom_media, false)

    Vize.Native.parse_css_ast_nif(source, filename, custom_media, css_modules)
  end

  @doc "Like `parse_ast/2` but raises on errors."
  @spec parse_ast!(String.t(), keyword()) :: ast_result()
  def parse_ast!(source, opts \\ []) do
    case parse_ast(source, opts) do
      {:ok, result} ->
        if result.errors != [] do
          raise "Vize CSS parse error: #{inspect(result.errors)}"
        end

        result
    end
  end

  @doc """
  Print CSS from an AST returned by `parse_ast/2`.

  ## Options

    * `:minify` — minify the output (default: `false`)
    * `:targets` — browser targets for autoprefixing, map with optional
      `:chrome`, `:firefox`, `:safari` keys as major version integers

  ## Examples

      iex> {:ok, parsed} = Vize.CSS.parse_ast(".foo { color: red }")
      iex> {:ok, printed} = Vize.CSS.print_ast(parsed.ast)
      iex> printed.code =~ "color"
      true
  """
  @spec print_ast(map(), keyword()) :: {:ok, css_result()}
  def print_ast(ast, opts \\ []) do
    minify = Keyword.get(opts, :minify, false)
    targets = Keyword.get(opts, :targets, %{})
    chrome = Map.get(targets, :chrome, -1)
    firefox = Map.get(targets, :firefox, -1)
    safari = Map.get(targets, :safari, -1)

    Vize.Native.print_css_ast_nif(ast, minify, chrome, firefox, safari)
  end

  @doc "Like `print_ast/2` but raises on errors."
  @spec print_ast!(map(), keyword()) :: css_result()
  def print_ast!(ast, opts \\ []) do
    case print_ast(ast, opts) do
      {:ok, result} ->
        if result.errors != [] do
          raise "Vize CSS print error: #{inspect(result.errors)}"
        end

        result
    end
  end

  @doc """
  Traverse the AST in pre-order and call `fun` for every map node.

  Returns `:ok`. Use `prewalk/2` or `postwalk/2` when you need to transform
  the AST.
  """
  @spec walk(ast(), (map() -> any())) :: :ok
  def walk(value, fun) when is_function(fun, 1) do
    prewalk(value, fn
      node when is_map(node) ->
        fun.(node)
        node

      other ->
        other
    end)

    :ok
  end

  @doc """
  Depth-first pre-order traversal, like `Macro.prewalk/2`.

  The callback receives every map node before its children and must return the
  node to continue traversing.
  """
  @spec prewalk(ast(), (map() -> map())) :: ast()
  def prewalk(value, fun) when is_function(fun, 1) do
    do_prewalk(value, fn node, acc -> {fun.(node), acc} end, nil) |> elem(0)
  end

  @doc """
  Depth-first pre-order traversal with accumulator, like `Macro.prewalk/3`.
  """
  @spec prewalk(ast(), acc, (map(), acc -> {map(), acc})) :: {ast(), acc} when acc: term()
  def prewalk(value, acc, fun) when is_function(fun, 2) do
    do_prewalk(value, fun, acc)
  end

  @doc """
  Depth-first post-order traversal, like `Macro.postwalk/2`.

  The callback receives every map node after its children and must return the
  transformed node.
  """
  @spec postwalk(ast(), (map() -> map())) :: ast()
  def postwalk(value, fun) when is_function(fun, 1) do
    do_postwalk(value, fn node, acc -> {fun.(node), acc} end, nil) |> elem(0)
  end

  @doc """
  Depth-first post-order traversal with accumulator, like `Macro.postwalk/3`.
  """
  @spec postwalk(ast(), acc, (map(), acc -> {map(), acc})) :: {ast(), acc} when acc: term()
  def postwalk(value, acc, fun) when is_function(fun, 2) do
    do_postwalk(value, fun, acc)
  end

  @doc """
  Collect values from map nodes that match `fun`.

  `fun` should return `{:keep, value}` to include a value, or `:skip` to ignore
  the node.
  """
  @spec collect(ast(), (map() -> {:keep, term()} | :skip)) :: [term()]
  def collect(value, fun) when is_function(fun, 1) do
    {_value, collected} =
      postwalk(value, [], fn node, acc ->
        case fun.(node) do
          {:keep, value} -> {node, [value | acc]}
          :skip -> {node, acc}
        end
      end)

    Enum.reverse(collected)
  end

  defp do_prewalk(value, fun, acc) when is_map(value) do
    {value, acc} = fun.(value, acc)

    Enum.reduce(value, {value, acc}, fn {key, child}, {node, acc} ->
      {child, acc} = do_prewalk(child, fun, acc)
      {Map.put(node, key, child), acc}
    end)
  end

  defp do_prewalk(value, fun, acc) when is_list(value) do
    Enum.map_reduce(value, acc, &do_prewalk(&1, fun, &2))
  end

  defp do_prewalk(value, _fun, acc), do: {value, acc}

  defp do_postwalk(value, fun, acc) when is_map(value) do
    {value, acc} =
      Enum.reduce(value, {value, acc}, fn {key, child}, {node, acc} ->
        {child, acc} = do_postwalk(child, fun, acc)
        {Map.put(node, key, child), acc}
      end)

    fun.(value, acc)
  end

  defp do_postwalk(value, fun, acc) when is_list(value) do
    Enum.map_reduce(value, acc, &do_postwalk(&1, fun, &2))
  end

  defp do_postwalk(value, _fun, acc), do: {value, acc}
end
