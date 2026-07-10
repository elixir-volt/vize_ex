use RustQ.Config

alias RustQ.Rustler

atoms =
  "native/vize_ex_nif/src/*.rs"
  |> Path.wildcard()
  |> Enum.reject(&String.ends_with?(&1, "/generated_atoms.rs"))
  |> Enum.flat_map(fn path -> path |> File.read!() |> RustQ.Syn.atom_references!() end)
  |> Enum.uniq()
  |> Enum.sort()
  |> Enum.map(fn
    "end_" -> {"end_", "end"}
    name -> name
  end)

rust "native/vize_ex_nif/src/generated_atoms.rs" do
  Rustler.atoms(atoms)
end
