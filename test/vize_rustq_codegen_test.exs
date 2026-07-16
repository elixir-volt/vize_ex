Code.require_file("../codegen/vize/codegen/native_types.ex", __DIR__)

defmodule Vize.RustQCodegenTest do
  use RustQ.Test, async: true

  test "derives the source-location NIF map for the existing crate" do
    source = RustQ.Native.source(Vize.Codegen.NativeTypes)

    assert source =~ "pub struct EncodedLoc"
    assert source =~ "rustler::NifMap"
    assert source =~ "pub end_column: usize"
    assert_rust_valid(Vize.Codegen.NativeTypes)
  end
end
