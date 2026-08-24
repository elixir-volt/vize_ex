defmodule Vize.Native do
  version = Mix.Project.config()[:version]

  use RustlerPrecompiled,
    otp_app: :vize,
    crate: "vize_ex_nif",
    base_url: "https://github.com/elixir-volt/vize_ex/releases/download/v#{version}",
    force_build: System.get_env("VIZE_EX_BUILD") in ["1", "true"],
    targets: ~w(
      aarch64-apple-darwin
      aarch64-unknown-linux-gnu
      x86_64-apple-darwin
      x86_64-pc-windows-msvc
      x86_64-unknown-linux-gnu
      x86_64-unknown-linux-musl
    ),
    version: version

  @spec parse_sfc_nif(String.t()) :: {:ok, map()} | {:error, String.t()}

  @spec analyze_sfc_nif(String.t(), String.t()) :: {:ok, map()} | {:error, String.t()}

  @spec compile_sfc_nif(
          String.t(),
          String.t(),
          String.t(),
          boolean(),
          boolean(),
          boolean(),
          boolean(),
          boolean()
        ) :: {:ok, map()} | {:error, String.t()}

  @spec sfc_template_assets_nif(String.t(), String.t()) :: [map()]

  @spec rewrite_sfc_template_assets_nif(String.t(), [{String.t(), String.t()}]) :: String.t()

  @spec sfc_src_info_nif(String.t(), String.t()) :: map()

  @spec sfc_scope_id_nif(String.t(), String.t(), boolean(), String.t()) :: String.t()

  @spec compile_template_nif(String.t(), String.t(), boolean()) ::
          {:ok, map()} | {:error, list()}

  @spec compile_ssr_nif(String.t()) :: {:ok, map()} | {:error, list()}

  @spec compile_vapor_nif(String.t(), boolean(), boolean(), String.t()) ::
          {:ok, map()} | {:error, list()}

  @spec vapor_ir_nif(String.t()) :: {:ok, map()} | {:error, list()}

  @spec vapor_split_nif(String.t()) :: {:ok, map()} | {:error, list()}

  @spec lint_nif(String.t(), String.t()) :: {:ok, list()}

  @spec select_css_nif(String.t(), String.t(), boolean(), boolean(), list()) ::
          {:ok, [map()]} | {:error, [String.t()]}

  @spec parse_css_ast_nif(String.t(), String.t(), boolean(), boolean()) :: {:ok, map()}

  @spec print_css_ast_nif(map(), boolean(), integer(), integer(), integer()) :: {:ok, map()}

  @spec compile_css_nif(
          String.t(),
          boolean(),
          boolean(),
          String.t(),
          String.t(),
          integer(),
          integer(),
          integer(),
          boolean()
        ) :: {:ok, map()}

  @spec bundle_css_nif(
          String.t(),
          boolean(),
          integer(),
          integer(),
          integer(),
          boolean()
        ) :: {:ok, map()}

  @spec generate_dts_nif(String.t(), String.t()) :: {:ok, map()} | {:error, String.t()}
  use Vize.GeneratedNifStubs
end
