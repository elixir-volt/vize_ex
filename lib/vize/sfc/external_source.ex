defmodule Vize.SFC.ExternalSource do
  @moduledoc "An SFC block whose content is loaded from an external source."

  @enforce_keys [:type, :src]
  defstruct [:type, :src, :index, :block_type]

  @type block_type :: :template | :script | :script_setup | :style | :custom

  @type t :: %__MODULE__{
          type: block_type(),
          src: String.t(),
          index: non_neg_integer() | nil,
          block_type: String.t() | nil
        }
end
