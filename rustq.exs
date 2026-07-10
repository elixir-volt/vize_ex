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

rust "native/vize_ex_nif/src/generated_term_encoders.rs" do
  [
    Rustler.term_encoder(:EncodedLoc,
      fields: [:start, {:end_, :end}, :start_line, :start_column, :end_line, :end_column]
    ),
    Rustler.term_encoder(:EncodedLintDiagnostic,
      fields: [:message, :name],
      target_lifetimes: [:_]
    ),
    Rustler.term_encoder(:EncodedSfcError,
      fields: [:message, code: [when_some: true]],
      target_lifetimes: [:_]
    )
  ]
end
