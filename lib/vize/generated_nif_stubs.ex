defmodule Vize.GeneratedNifStubs do
  @moduledoc false
  defmacro __using__(_opts) do
    quote do
      def parse_sfc_nif(_source) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def analyze_sfc_nif(_source, _mode) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def compile_sfc_nif(
            _source,
            _filename,
            _scope_id,
            _vapor,
            _ssr,
            _custom_renderer,
            _strip_types
          ) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def compile_template_nif(_source, _mode, _ssr) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def compile_ssr_nif(_source) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def compile_vapor_nif(_source, _ssr, _diagnostics, _template_syntax) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def vapor_ir_nif(_source) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def lint_nif(_source, _filename) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def select_css_nif(_source, _filename, _custom_media, _css_modules, _selector_term) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def parse_css_ast_nif(_source, _filename, _custom_media, _css_modules) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def print_css_ast_nif(_ast, _minify, _chrome, _firefox, _safari) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def compile_sass_nif(_source, _syntax, _filename, _load_paths, _compressed) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def compile_css_nif(
            _source,
            _minify,
            _scoped,
            _scope_id_str,
            _filename,
            _chrome,
            _firefox,
            _safari,
            _css_modules
          ) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def bundle_css_nif(_entry_path, _minify, _chrome, _firefox, _safari, _css_modules) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def vapor_split_nif(_source) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def generate_dts_nif(_source, _filename) do
        :erlang.nif_error(:nif_not_loaded)
      end
    end
  end
end
