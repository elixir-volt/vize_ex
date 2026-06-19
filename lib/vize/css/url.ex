defmodule Vize.CSS.URL do
  @moduledoc "A parser-backed CSS `url()` reference."

  defstruct [:url, :range, :location]

  @type t :: %__MODULE__{
          url: String.t(),
          range: Vize.Range.t(),
          location: Vize.SourceRange.t()
        }

  @spec new(map()) :: t()
  def new(map) when is_map(map) do
    %__MODULE__{
      url: map[:url] || map["url"],
      range: %Vize.Range{start: map[:start] || map["start"], end: map[:end] || map["end"]},
      location: %Vize.SourceRange{
        start: %Vize.SourceLocation{
          line: map[:start_line] || map["start_line"],
          column: map[:start_column] || map["start_column"]
        },
        end: %Vize.SourceLocation{
          line: map[:end_line] || map["end_line"],
          column: map[:end_column] || map["end_column"]
        }
      }
    }
  end
end
