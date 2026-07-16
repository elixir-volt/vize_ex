defmodule Vize.Codegen.NativeTypes do
  @moduledoc false

  use RustQ.Native,
    build: false,
    load: false,
    crate: :vize_native_types

  alias RustQ.Type, as: R

  @type encoded_loc :: %{
          required(:start) => R.usize(),
          required(:end) => R.usize(),
          required(:start_line) => R.usize(),
          required(:start_column) => R.usize(),
          required(:end_line) => R.usize(),
          required(:end_column) => R.usize()
        }
end
