defmodule Vize.Codegen.Sfc do
  @moduledoc false

  # Term encoders for SFC descriptor types, generated from Vize's Rust source.

  alias RustQ.Rustler.Term

  @roots ["BlockLocation"]

  def encoders, do: Term.encoders_from_source(index(), @roots, [])

  def atoms, do: Term.encoder_atoms_from_source(index(), @roots, [])

  defp index do
    RustQ.Syn.Index.cached_package("vize_croquis", manifest_path: "native/vize_ex_nif/Cargo.toml")
  end
end
