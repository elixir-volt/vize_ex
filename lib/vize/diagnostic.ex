defmodule Vize.Diagnostic do
  @moduledoc """
  Structured diagnostic emitted by Vize.

  `to_code_diagnostic/2` converts one to Elixir's `t:Code.diagnostic/1` shape,
  which editors, `Mix.Task.Compiler`, and tools such as OXC and Volt share.
  """

  defstruct [:code, :message, :location, severity: :error, recoverable?: false]

  @type t :: %__MODULE__{
          code: atom() | String.t() | nil,
          message: String.t(),
          location: Vize.SourceRange.t() | nil,
          severity: :error | :warning,
          recoverable?: boolean()
        }

  @type input :: %{
          optional(:code) => atom() | String.t() | nil,
          required(:message) => String.t(),
          optional(:location) => Vize.SourceRange.input() | Vize.SourceRange.t() | nil,
          optional(:loc) => Vize.SourceRange.input() | Vize.SourceRange.t() | nil,
          optional(:start) => Vize.SourceLocation.input() | nil,
          optional(:end) => Vize.SourceLocation.input() | nil,
          optional(:severity) => :error | :warning,
          optional(:recoverable?) => boolean(),
          optional(:recoverable) => boolean()
        }

  @spec new(input() | t() | String.t()) :: t()
  def new(%__MODULE__{} = diagnostic), do: diagnostic
  def new(message) when is_binary(message), do: %__MODULE__{message: message}

  def new(%{} = attrs) do
    location =
      Map.get(attrs, :location) || Map.get(attrs, :loc) ||
        if Map.has_key?(attrs, :start), do: %{start: attrs.start, end: Map.get(attrs, :end)}

    %__MODULE__{
      code: Map.get(attrs, :code),
      message: Map.fetch!(attrs, :message),
      location: Vize.SourceRange.new(location),
      severity: Map.get(attrs, :severity, :error),
      recoverable?: Map.get(attrs, :recoverable?, Map.get(attrs, :recoverable, false))
    }
  end

  @doc """
  Converts a diagnostic to Elixir's `t:Code.diagnostic/1` shape.

  ## Options

    * `:file` — the file the diagnostic belongs to
    * `:origin` — `{line, column}` where the source Vize was given starts in
      that file, such as a `<template>` block's content in an SFC, so
      positions point into the file (default: `{1, 1}`)
  """
  @spec to_code_diagnostic(t(), keyword()) :: Code.diagnostic(:error | :warning)
  def to_code_diagnostic(%__MODULE__{} = diagnostic, opts \\ []) do
    origin = Keyword.get(opts, :origin, {1, 1})
    location = diagnostic.location

    %{
      file: Keyword.get(opts, :file),
      severity: diagnostic.severity,
      message: diagnostic.message,
      position: position(location && location.start, origin) || 0,
      span: position(location && location.end, origin),
      source: Keyword.get(opts, :file),
      stacktrace: [],
      details: nil
    }
  end

  @doc """
  Shifts a `{line, column}` position in a source Vize was given to the file it
  came from, whose content starts at `origin`.
  """
  @spec shift({pos_integer(), pos_integer()}, {pos_integer(), pos_integer()}) ::
          {pos_integer(), pos_integer()}
  def shift({1, column}, {origin_line, origin_column}),
    do: {origin_line, origin_column + column - 1}

  def shift({line, column}, {origin_line, _origin_column}), do: {origin_line + line - 1, column}

  defp position(%Vize.SourceLocation{line: line, column: column}, origin)
       when is_integer(line) and is_integer(column),
       do: shift({line, column}, origin)

  defp position(_location, _origin), do: nil
end
