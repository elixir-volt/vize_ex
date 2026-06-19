defmodule Vize.SourceLocation do
  @moduledoc "A source position reported by Vize."

  defstruct [:offset, :line, :column]

  @type t :: %__MODULE__{
          offset: non_neg_integer() | nil,
          line: pos_integer() | nil,
          column: pos_integer() | nil
        }

  @spec new(map() | nil) :: t() | nil
  def new(nil), do: nil

  def new(map) when is_map(map) do
    %__MODULE__{
      offset: map[:offset] || map["offset"],
      line: map[:line] || map["line"],
      column: map[:column] || map["column"]
    }
  end
end
