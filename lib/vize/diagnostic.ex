defmodule Vize.Diagnostic do
  @moduledoc "Structured diagnostic emitted by Vize."

  defstruct [:code, :message, :location, recoverable?: false]

  @type t :: %__MODULE__{
          code: atom() | String.t() | nil,
          message: String.t(),
          location: Vize.SourceRange.t() | nil,
          recoverable?: boolean()
        }

  @spec new(map() | String.t()) :: t()
  def new(message) when is_binary(message), do: %__MODULE__{message: message}

  def new(map) when is_map(map) do
    %__MODULE__{
      code: map[:code] || map["code"],
      message: map[:message] || map["message"],
      location:
        Vize.SourceRange.new(map[:location] || map["location"] || map[:loc] || map["loc"]),
      recoverable?: map[:recoverable?] || map["recoverable"] || map[:recoverable] || false
    }
  end
end
