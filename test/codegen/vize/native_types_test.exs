Code.require_file("../../../codegen/vize/codegen/native_types.ex", __DIR__)
Code.require_file("../../../codegen/vize/codegen/sfc.ex", __DIR__)

defmodule Vize.Codegen.NativeTypesTest do
  use RustQ.Test, async: true

  test "derives NIF option maps for the existing crate" do
    source = RustQ.Native.source(Vize.Codegen.NativeTypes)

    assert source =~ "pub struct CompileSfcOpts"
    assert source =~ "pub targets: BrowserTargets"
    assert source =~ "pub chrome: Option<u32>"
    assert source =~ "rustler::NifMap"
    assert RustQ.valid?(source, "vize_native_types.rs")
  end

  test "encodes SFC block locations from Vize's source" do
    source = Vize.Codegen.Sfc.encoders() |> RustQ.Rust.render_all()

    assert source =~ "fn encode_block_location"
    assert source =~ "value.tag_start"
  end
end
