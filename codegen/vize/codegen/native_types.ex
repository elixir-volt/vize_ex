defmodule Vize.Codegen.NativeTypes do
  @moduledoc false

  # Result shapes returned by the NIFs. RustQ derives a Rust struct and its
  # Term codec for each type.

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

  ## Diagnostics

  @type encoded_position :: %{
          required(:offset) => R.u32(),
          required(:line) => R.u32(),
          required(:column) => R.u32()
        }

  @type encoded_source_location :: %{
          required(:start) => encoded_position(),
          required(:end) => encoded_position(),
          required(:source) => String.t()
        }

  @type encoded_diagnostic :: %{
          required(:code) => String.t() | nil,
          required(:message) => String.t(),
          required(:recoverable) => boolean(),
          required(:location) => encoded_source_location() | nil
        }

  ## Vapor

  @type encoded_vapor_output :: %{
          required(:code) => String.t(),
          required(:templates) => [String.t()]
        }

  @type encoded_vapor_diagnostics_output :: %{
          required(:code) => String.t(),
          required(:templates) => [String.t()],
          required(:diagnostics) => [encoded_diagnostic()]
        }

  @type encoded_vapor_ir :: %{
          required(:templates) => [String.t()],
          required(:components) => [String.t()],
          required(:directives) => [String.t()],
          required(:block) => R.term(),
          required(:element_template_map) => [{R.usize(), R.usize()}]
        }

  @type encoded_vapor_split :: %{
          required(:statics) => [String.t()],
          required(:slots) => [R.term()],
          required(:templates) => [String.t()],
          required(:element_template_map) => [{R.usize(), R.usize()}]
        }

  ## vapor_split slots
  #
  # Each slot fills the hole between two statics. Expressions (`R.term()`) are
  # a string, or `{:static, string}` for static values.

  @type encoded_split_block :: %{
          required(:statics) => [String.t()],
          required(:slots) => [R.term()]
        }

  @type encoded_values_slot :: %{
          required(:kind) => :set_prop | :set_text,
          required(:values) => [R.term()]
        }

  @type encoded_value_slot :: %{
          required(:kind) => :v_show | :v_model | :set_html,
          required(:value) => R.term()
        }

  # `negative` is a split block, a nested if slot, or nil.
  @type encoded_if_slot :: %{
          required(:kind) => :if_node,
          required(:condition) => R.term(),
          required(:positive) => encoded_split_block(),
          required(:negative) => R.term() | nil
        }

  @type encoded_for_slot :: %{
          required(:kind) => :for_node,
          required(:source) => R.term(),
          required(:value) => R.term() | nil,
          required(:key_prop) => R.term() | nil,
          required(:render) => encoded_split_block()
        }

  @type encoded_component_slot :: %{
          required(:kind) => :create_component,
          required(:tag) => String.t(),
          required(:props) => [R.term()],
          required(:value) =>
            :regular
            | :teleport
            | :keep_alive
            | :suspense
            | :transition
            | :transition_group
            | :dynamic
        }

  ## SFC

  @type encoded_template_asset :: %{
          required(:url) => String.t(),
          required(:var_name) => String.t()
        }

  @type encoded_dts :: %{required(:dts) => String.t()}

  @type encoded_sass :: %{required(:code) => String.t()}

  ## SFC analysis

  @type encoded_sfc_analysis :: %{
          required(:stats) => encoded_sfc_stats(),
          required(:bindings) => [encoded_binding()],
          required(:props) => [encoded_prop_declaration()],
          required(:emits) => [String.t()],
          required(:models) => [String.t()],
          required(:used_components) => [String.t()],
          required(:used_directives) => [String.t()],
          required(:undefined_refs) => [encoded_undefined_ref()],
          required(:component_usages) => [encoded_component_usage()],
          required(:template_expressions) => [encoded_template_expression()]
        }

  @type encoded_sfc_stats :: %{
          required(:bindings) => R.usize(),
          required(:props) => R.usize(),
          required(:emits) => R.usize(),
          required(:models) => R.usize(),
          required(:used_components) => R.usize(),
          required(:used_directives) => R.usize(),
          required(:undefined_refs) => R.usize()
        }

  @type encoded_binding :: %{required(:name) => String.t(), required(:kind) => String.t()}

  @type encoded_prop_declaration :: %{
          required(:name) => String.t(),
          required(:required) => boolean()
        }

  @type encoded_undefined_ref :: %{
          required(:name) => String.t(),
          required(:offset) => R.u32(),
          required(:context) => String.t()
        }

  @type encoded_component_usage :: %{
          required(:name) => String.t(),
          required(:props) => [encoded_passed_prop()],
          required(:events) => [encoded_event_listener()],
          required(:has_spread_attrs) => boolean()
        }

  @type encoded_passed_prop :: %{
          required(:name) => String.t(),
          required(:value) => String.t() | nil,
          required(:is_dynamic) => boolean()
        }

  @type encoded_event_listener :: %{
          required(:name) => String.t(),
          required(:handler) => String.t() | nil
        }

  @type encoded_template_expression :: %{
          required(:source) => String.t(),
          required(:kind) => String.t(),
          required(:range) => encoded_range()
        }

  @type encoded_range :: %{required(:start) => R.u32(), required(:end) => R.u32()}
end
