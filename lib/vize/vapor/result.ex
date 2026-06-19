defmodule Vize.Vapor.Result do
  @moduledoc "Result of Vapor template compilation."

  defstruct [:code, templates: [], diagnostics: []]

  @type t :: %__MODULE__{
          code: String.t(),
          templates: [String.t()],
          diagnostics: [Vize.Diagnostic.t()]
        }

  @spec new(map()) :: t()
  def new(map) when is_map(map) do
    %__MODULE__{
      code: map[:code] || map["code"],
      templates: map[:templates] || map["templates"] || [],
      diagnostics: Enum.map(map[:diagnostics] || map["diagnostics"] || [], &Vize.Diagnostic.new/1)
    }
  end
end
