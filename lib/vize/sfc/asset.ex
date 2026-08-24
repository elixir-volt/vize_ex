defmodule Vize.SFC.Asset do
  @moduledoc "A static template asset and its generated JavaScript binding."

  @enforce_keys [:url, :binding]
  defstruct [:url, :binding]

  @type t :: %__MODULE__{
          url: String.t(),
          binding: String.t()
        }

  @doc false
  @spec new(%{required(:url) => String.t(), required(:var_name) => String.t()}) :: t()
  def new(%{url: url, var_name: binding}) do
    %__MODULE__{url: url, binding: binding}
  end
end
