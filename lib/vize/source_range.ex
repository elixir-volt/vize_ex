defmodule Vize.SourceRange do
  @moduledoc "A source span reported by Vize."

  defstruct [:start, :end, :source]

  @type t :: %__MODULE__{
          start: Vize.SourceLocation.t() | nil,
          end: Vize.SourceLocation.t() | nil,
          source: String.t() | nil
        }

  @spec new(map() | nil) :: t() | nil
  def new(nil), do: nil

  def new(map) when is_map(map) do
    %__MODULE__{
      start: Vize.SourceLocation.new(map[:start] || map["start"]),
      end: Vize.SourceLocation.new(map[:end] || map["end"]),
      source: map[:source] || map["source"]
    }
  end
end
