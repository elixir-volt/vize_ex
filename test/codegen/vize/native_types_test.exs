Code.require_file("../../../codegen/vize/codegen/native_types.ex", __DIR__)

defmodule Vize.Codegen.NativeTypesTest do
  use RustQ.Test, async: true

  test "derives the source-location NIF map for the existing crate" do
    source = RustQ.Native.source(Vize.Codegen.NativeTypes)

    assert source =~ "pub struct EncodedLoc"
    assert source =~ "rustler::NifMap"
    assert source =~ "pub end_column: usize"
    assert RustQ.valid?(source, "vize_native_types.rs")
  end
end
