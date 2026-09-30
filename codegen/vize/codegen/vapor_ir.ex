defmodule Vize.Codegen.VaporIR do
  @moduledoc false

  # Term encoders for `vize_atelier_vapor`'s IR, generated from its Rust source.
  # Everything below is vize_ex policy: the shape `Vize.vapor_ir/1` returns.

  alias RustQ.Rustler.Term

  @roots ["BlockIRNode"]

  def encoders, do: Term.encoders_from_source(index(), @roots, opts())

  def atoms, do: Term.encoder_atoms_from_source(index(), @roots, opts())

  defp index do
    RustQ.Syn.Index.cached_package("vize_atelier_vapor",
      manifest_path: "native/vize_ex_nif/Cargo.toml"
    )
  end

  defp opts do
    [
      tag: :kind,
      external: [
        SimpleExpressionNode: :encode_simple_expr,
        InsertionAnchor: :encode_insertion_anchor
      ],
      types: [
        # The template AST node is not part of the IR output.
        BlockIRNode: [
          except: [:node],
          fields: [operation: [key: :operations], effect: [key: :effects]]
        ],
        IREffect: [transparent: true],
        OperationNode: [variants: [If: :if_node, For: :for_node]],
        NegativeBranch: [variants: [If: :if_node]],
        # Keys kept from the handwritten encoders that preceded generation.
        SetPropIRNode: [fields: [prop: [key: :value]]],
        InsertNodeIRNode: [fields: [elements: [key: :element]]],
        PrependNodeIRNode: [fields: [elements: [key: :element]]],
        NextRefIRNode: [fields: [prev_id: [key: :parent_id]]],
        CreateComponentIRNode: [fields: [kind: [key: :value]]],
        DirectiveIRNode: [fields: [dir: [key: :value, with: :encode_directive_expression]]]
      ]
    ]
  end
end
