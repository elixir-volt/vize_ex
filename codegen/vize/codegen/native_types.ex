defmodule Vize.Codegen.NativeTypes do
  @moduledoc false

  # Options passed to and results returned by the NIFs. RustQ derives a Rust
  # struct and its Term codec for each type. Option defaults live in the public
  # Elixir functions, which always pass every key.

  use RustQ.Native,
    build: false,
    load: false,
    crate: :vize_native_types

  alias RustQ.Type, as: R

  ## NIF options

  @type compile_sfc_opts :: %{
          required(:filename) => String.t(),
          required(:scope_id) => String.t(),
          required(:vapor) => boolean(),
          required(:ssr) => boolean(),
          required(:custom_renderer) => boolean(),
          required(:strip_types) => boolean(),
          required(:source_map) => boolean()
        }

  @type scope_id_opts :: %{
          required(:root) => String.t(),
          required(:production) => boolean(),
          required(:source) => String.t()
        }

  @type compile_template_opts :: %{
          required(:mode) => String.t(),
          required(:ssr) => boolean()
        }

  @type compile_vapor_opts :: %{
          required(:ssr) => boolean(),
          required(:diagnostics) => boolean(),
          required(:template_syntax) => String.t()
        }

  @type browser_targets :: %{
          required(:chrome) => R.u32() | nil,
          required(:firefox) => R.u32() | nil,
          required(:safari) => R.u32() | nil
        }

  @type compile_css_opts :: %{
          required(:minify) => boolean(),
          required(:scoped) => boolean(),
          required(:scope_id) => String.t(),
          required(:filename) => String.t(),
          required(:targets) => browser_targets(),
          required(:css_modules) => boolean()
        }

  @type bundle_css_opts :: %{
          required(:minify) => boolean(),
          required(:targets) => browser_targets(),
          required(:css_modules) => boolean()
        }

  @type parse_css_opts :: %{
          required(:filename) => String.t(),
          required(:custom_media) => boolean(),
          required(:css_modules) => boolean()
        }

  @type print_css_opts :: %{
          required(:minify) => boolean(),
          required(:targets) => browser_targets()
        }

  @type compile_sass_opts :: %{
          required(:syntax) => String.t(),
          required(:filename) => String.t(),
          required(:load_paths) => [String.t()],
          required(:compressed) => boolean()
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

  ## split_template
  #
  # A template split into static HTML and the dynamic slots between them, the
  # shape of `%Phoenix.LiveView.Rendered{}`. Expressions are JavaScript source
  # strings. Every slot has a `position`, `{line, column}` in the template.

  @type split_template_opts :: %{required(:root_attrs) => boolean()}

  @type encoded_template_split :: %{
          required(:statics) => [String.t()],
          required(:slots) => [R.term()],
          required(:bindings) => [encoded_split_binding()],
          required(:diagnostics) => [encoded_split_diagnostic()]
        }

  @type encoded_split_block :: %{
          required(:statics) => [String.t()],
          required(:slots) => [R.term()],
          required(:bindings) => [encoded_split_binding()]
        }

  # A problem with the template. Positions are 1-based; `end` is exclusive.
  @type encoded_split_diagnostic :: %{
          required(:severity) => :error | :warning,
          required(:message) => String.t(),
          required(:start) => encoded_position(),
          required(:end) => encoded_position()
        }

  # An event or `v-model` on an element, left for the caller to render. `at` is
  # `{static_index, byte_offset}`: where the element's start tag ends.
  @type encoded_split_binding :: %{
          required(:kind) => :on | :model,
          required(:name) => String.t() | nil,
          required(:modifiers) => [String.t()],
          required(:value) => String.t() | nil,
          required(:at) => {R.usize(), R.usize()},
          required(:position) => {R.usize(), R.usize()}
        }

  # Escaped text from an interpolation.
  @type encoded_text_slot :: %{
          required(:kind) => :text,
          required(:value) => String.t(),
          required(:position) => {R.usize(), R.usize()}
        }

  # Unescaped HTML, from `v-html`.
  @type encoded_html_slot :: %{
          required(:kind) => :html,
          required(:value) => String.t(),
          required(:position) => {R.usize(), R.usize()}
        }

  # One attribute, rendered whole with its leading space so a caller can leave
  # it out. `name_value` is the expression of a dynamic `:[name]`. `static` is
  # a static attribute of the same name, such as `class="a"` beside
  # `:class="b"`, and `show` the `v-show` expression a `style` combines with.
  @type encoded_attr_slot :: %{
          required(:kind) => :attr,
          required(:name) => String.t() | nil,
          required(:name_value) => String.t() | nil,
          required(:static) => String.t() | nil,
          required(:value) => String.t() | nil,
          required(:show) => String.t() | nil,
          required(:position) => {R.usize(), R.usize()}
        }

  # The attributes of an object, from `v-bind="attrs"`.
  @type encoded_spread_slot :: %{
          required(:kind) => :spread,
          required(:value) => String.t(),
          required(:position) => {R.usize(), R.usize()}
        }

  # A `v-model` on a form element: the `value`, `checked` state, or text it
  # renders. `type` and `static_value` are the element's static `type` and
  # `value` attributes, which decide that for checkboxes and radios.
  @type encoded_model_slot :: %{
          required(:kind) => :model,
          required(:tag) => String.t(),
          required(:type) => String.t() | nil,
          required(:static_value) => String.t() | nil,
          required(:value) => String.t(),
          required(:position) => {R.usize(), R.usize()}
        }

  @type encoded_if_slot :: %{
          required(:kind) => :if,
          required(:branches) => [encoded_if_branch()],
          required(:position) => {R.usize(), R.usize()}
        }

  # `condition` is nil for `v-else`.
  @type encoded_if_branch :: %{
          required(:condition) => String.t() | nil,
          required(:block) => encoded_split_block()
        }

  @type encoded_for_slot :: %{
          required(:kind) => :for,
          required(:source) => String.t(),
          required(:value) => String.t(),
          required(:key) => String.t() | nil,
          required(:index) => String.t() | nil,
          required(:key_prop) => String.t() | nil,
          required(:block) => encoded_split_block(),
          required(:position) => {R.usize(), R.usize()}
        }

  # A prop, attribute or `v-bind` object passed to a component or outlet, or
  # one of a root element's attributes. A spread has neither name.
  @type encoded_prop :: %{
          required(:name) => String.t() | nil,
          required(:name_value) => String.t() | nil,
          required(:static) => String.t() | nil,
          required(:value) => String.t() | nil
        }

  @type encoded_event :: %{
          required(:name) => String.t() | nil,
          required(:modifiers) => [String.t()],
          required(:value) => String.t() | nil
        }

  @type encoded_component_slot :: %{
          required(:kind) => :component,
          required(:name) => String.t(),
          required(:props) => [encoded_prop()],
          required(:events) => [encoded_event()],
          required(:slots) => [encoded_slot_content()],
          required(:position) => {R.usize(), R.usize()}
        }

  # Content passed to one of a component's slots. `params` is the slot props
  # pattern, such as `{ item }`.
  @type encoded_slot_content :: %{
          required(:name) => String.t() | nil,
          required(:name_value) => String.t() | nil,
          required(:params) => String.t() | nil,
          required(:block) => encoded_split_block()
        }

  # A `<slot>` outlet, where the parent's slot content renders.
  @type encoded_slot_outlet :: %{
          required(:kind) => :slot,
          required(:name) => String.t() | nil,
          required(:name_value) => String.t() | nil,
          required(:props) => [encoded_prop()],
          required(:fallback) => encoded_split_block() | nil,
          required(:position) => {R.usize(), R.usize()}
        }

  # With `root_attrs: true`, every attribute of the template's single root
  # element, in authored order, so a caller can merge a component's
  # fallthrough attributes in.
  @type encoded_root_attrs_slot :: %{
          required(:kind) => :root_attrs,
          required(:props) => [encoded_prop()],
          required(:show) => String.t() | nil,
          required(:position) => {R.usize(), R.usize()}
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
