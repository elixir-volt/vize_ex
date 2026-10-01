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

      def compile_sfc_nif(_source, _opts) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def sfc_template_assets_nif(_source, _filename) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def rewrite_sfc_template_assets_nif(_code, _assets) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def sfc_scope_id_nif(_filename, _opts) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def compile_template_nif(_source, _opts) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def compile_ssr_nif(_source) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def compile_vapor_nif(_source, _opts) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def vapor_ir_nif(_source) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def lint_nif(_source, _filename) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def select_css_nif(_source, _opts, _selector_term) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def parse_css_ast_nif(_source, _opts) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def print_css_ast_nif(_ast, _opts) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def compile_sass_nif(_source, _opts) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def compile_css_nif(_source, _opts) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def bundle_css_nif(_entry_path, _opts) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def vapor_split_nif(_source, _opts) do
        :erlang.nif_error(:nif_not_loaded)
      end

      def generate_dts_nif(_source, _filename) do
        :erlang.nif_error(:nif_not_loaded)
      end
    end
  end
end
