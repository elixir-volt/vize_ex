defmodule Vize.Error do
  @moduledoc "Exception returned or raised by Vize bang APIs."

  defexception [:message, diagnostics: [], errors: []]

  @type t :: %__MODULE__{
          message: String.t(),
          diagnostics: [Vize.Diagnostic.t()],
          errors: term()
        }
end
