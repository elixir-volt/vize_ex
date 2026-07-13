use RustQ.Config

alias RustQ.Rustler

encoders = [
  {:EncodedLoc,
   fields: [:start, {:end_, :end}, :start_line, :start_column, :end_line, :end_column]},
  {:EncodedLintDiagnostic, fields: [:message, :name], target_lifetimes: [:_]},
  {:EncodedSfcError,
   fields: [:message, code: [when_some: true]], target_lifetimes: [:_]},
  {:EncodedTemplateBlock,
   fields: [
     content: [field: [0, :content], via: :as_ref],
     lang: [field: [0, :lang], via: :as_deref],
     loc: [field: [0, :loc], with: :loc_to_term],
     attrs: [field: [0, :attrs], with: :attrs_to_term]
   ],
   target_lifetimes: [:_]},
  {:EncodedScriptBlock,
   fields: [
     content: [field: [0, :content], via: :as_ref],
     lang: [field: [0, :lang], via: :as_deref],
     setup: [field: [0, :setup]],
     loc: [field: [0, :loc], with: :loc_to_term],
     attrs: [field: [0, :attrs], with: :attrs_to_term]
   ],
   target_lifetimes: [:_]},
  {:EncodedStyleBlock,
   fields: [
     content: [field: [0, :content], via: :as_ref],
     lang: [field: [0, :lang], via: :as_deref],
     scoped: [field: [0, :scoped]],
     module: [field: [0, :module], via: :as_deref],
     loc: [field: [0, :loc], with: :loc_to_term],
     attrs: [field: [0, :attrs], with: :attrs_to_term]
   ],
   target_lifetimes: [:_]},
  {:EncodedCustomBlock,
   fields: [
     block_type: [field: [0, :block_type], via: :as_ref],
     content: [field: [0, :content], via: :as_ref],
     loc: [field: [0, :loc], with: :loc_to_term],
     attrs: [field: [0, :attrs], with: :attrs_to_term]
   ],
   target_lifetimes: [:_]},
  {:EncodedMacroArtifact,
   fields: [
     kind: [field: [0, :kind], via: :as_str],
     name: [field: [0, :name], via: :as_str],
     source: [field: [0, :source], via: :as_str],
     content: [field: [0, :content], via: :as_str],
     start: [field: [0, :start]],
     end_: [field: [0, :end]],
     code: [field: [0, :module_code], when_some: true, via: :as_str]
   ],
   target_lifetimes: [:_]},
  {:EncodedTemplateCompileResult,
   fields: [:code, :preamble, :helpers], target_lifetimes: [:_]},
  {:EncodedSsrCompileResult, fields: [:code, :preamble], target_lifetimes: [:_]}
]

encoder_atoms =
  Enum.flat_map(encoders, fn {_name, opts} ->
    Enum.map(Keyword.fetch!(opts, :fields), fn
      field when is_atom(field) -> Atom.to_string(field)
      {key, _field_or_opts} -> Atom.to_string(key)
    end)
  end)

source_atoms =
  "native/vize_ex_nif/src/*.rs"
  |> Path.wildcard()
  |> Enum.reject(&(Path.basename(&1) |> String.starts_with?("generated_")))
  |> Enum.flat_map(fn path -> path |> File.read!() |> RustQ.Syn.atom_references!() end)

atoms =
  (source_atoms ++ encoder_atoms)
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
  Enum.map(encoders, fn {name, opts} -> Rustler.term_encoder(name, opts) end)
end
