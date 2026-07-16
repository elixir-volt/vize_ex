use RustQ.Config

alias RustQ.Rust.AST.Builder, as: A
alias RustQ.Rustler.{Atom, Nif, Term}

unless Code.ensure_loaded?(Vize.Codegen.NativeTypes) do
  Code.require_file("codegen/vize/codegen/native_types.ex")
end

derived_encoders = [
  {:EncodedLoc,
   fields: [:start, {:end_, :end}, :start_line, :start_column, :end_line, :end_column]}
]

encoders = [
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
  {:EncodedSsrCompileResult, fields: [:code, :preamble], target_lifetimes: [:_]},
  {:EncodedParseSfcResult,
   fields: [
     template: [field: [:descriptor, :template], optional: [wrap: :EncodedTemplateBlock]],
     script: [field: [:descriptor, :script], optional: [wrap: :EncodedScriptBlock]],
     script_setup: [field: [:descriptor, :script_setup], optional: [wrap: :EncodedScriptBlock]],
     styles: [field: [:descriptor, :styles], map: [wrap: :EncodedStyleBlock]],
     custom_blocks: [field: [:descriptor, :custom_blocks], map: [wrap: :EncodedCustomBlock]]
   ],
   target_lifetimes: [:_]},
  {:EncodedCompileSfcResult,
   fields: [
     code: [field: :code_override, fallback: [field: [:result, :code], via: :as_str]],
     css: [field: [:result, :css], via: :as_deref],
     errors: [field: [:result, :errors], map: [convert: :EncodedSfcError]],
     warnings: [field: [:result, :warnings], map: [convert: :EncodedSfcError]],
     template_hash: [via: :as_deref],
     style_hash: [via: :as_deref],
     script_hash: [via: :as_deref],
     macro_artifacts: [field: [:result, :macro_artifacts], map: [wrap: :EncodedMacroArtifact]]
   ],
   target_lifetimes: [:_]},
  {:EncodedCssAstResult,
   fields: [
     ast: [field: [:result, :ast], optional: [with: :encode_json_value]],
     errors: [field: [:result, :errors], map: [via: :as_str]],
     warnings: [field: [:result, :warnings], map: [via: :as_str]]
   ],
   target_lifetimes: [:_]},
  {:EncodedCssCompileResult,
   fields: [
     code: [field: [:result, :code], via: :as_str],
     css_vars: [field: [:result, :css_vars], map: [via: :as_str]],
     errors: [field: [:result, :errors], map: [via: :as_str]],
     warnings: [field: [:result, :warnings], map: [via: :as_str]],
     exports: [field: [:result, :exports], via: :as_ref, with: :encode_css_exports, borrow: false]
   ],
   target_lifetimes: [:_]},
  {:EncodedBundleCssResult,
   fields: [
     code: [field: [:result, :code], via: :as_str],
     errors: [field: [:result, :errors], map: [via: :as_str]],
     warnings: [field: [:result, :warnings], map: [via: :as_str]],
     exports: [field: [:result, :exports], via: :as_ref, with: :encode_css_exports, borrow: false]
   ],
   target_lifetimes: [:_]}
]

nifs = [
  parse_sfc_nif: [],
  analyze_sfc_nif: [],
  compile_sfc_nif: [attrs: [A.attr(:allow, [A.path([:clippy, :too_many_arguments])])]],
  compile_template_nif: [],
  compile_ssr_nif: [],
  compile_vapor_nif: [],
  vapor_ir_nif: [],
  lint_nif: [],
  select_css_nif: [],
  parse_css_ast_nif: [],
  print_css_ast_nif: [],
  compile_sass_nif: [],
  compile_css_nif: [attrs: [A.attr(:allow, [A.path([:clippy, :too_many_arguments])])]],
  bundle_css_nif: [],
  vapor_split_nif: [],
  generate_dts_nif: []
]

encoder_atoms =
  Enum.flat_map(derived_encoders ++ encoders, fn {_name, opts} ->
    Term.encoder_atom_names(opts)
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
  Atom.declaration(atoms)
end

rust "native/vize_ex_nif/src/generated_types.rs" do
  RustQ.Native.items(Vize.Codegen.NativeTypes)
end

rust "native/vize_ex_nif/src/generated_term_encoders.rs" do
  Enum.map(encoders, fn {name, opts} -> Term.encoder(name, opts) end)
end

rust "native/vize_ex_nif/src/generated_nifs.rs" do
  Nif.wrappers_from_source(
    "native/vize_ex_nif/src/lib.rs",
    nifs,
    schedule: :dirty_cpu
  )
end

generate "lib/vize/generated_nif_stubs.ex" do
  content(
    Nif.stubs_from_source(
      "native/vize_ex_nif/src/lib.rs",
      nifs,
      Vize.GeneratedNifStubs
    )
  )
end
