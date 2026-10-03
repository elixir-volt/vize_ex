defmodule Vize.Error do
  @moduledoc "Exception returned or raised by Vize bang APIs."

  defexception [:message, diagnostics: [], errors: []]

  @type t :: %__MODULE__{
          message: String.t(),
          diagnostics: [Vize.Diagnostic.t()],
          errors: term()
        }

  @spec new(String.t(), term()) :: t()
  def new(message, errors) do
    diagnostics = Enum.map(List.wrap(errors), &Vize.Diagnostic.new/1)
    %__MODULE__{message: message, diagnostics: diagnostics, errors: errors}
  end

  @impl true
  def message(%__MODULE__{message: message, diagnostics: []}), do: message

  def message(%__MODULE__{message: message, diagnostics: diagnostics}) do
    Enum.map_join([message | Enum.map(diagnostics, &line/1)], "\n", & &1)
  end

  defp line(%Vize.Diagnostic{
         message: message,
         location: %Vize.SourceRange{start: %Vize.SourceLocation{line: line, column: column}}
       })
       when is_integer(line),
       do: "  #{line}:#{column}: #{message}"

  defp line(%Vize.Diagnostic{message: message}), do: "  " <> message
end
