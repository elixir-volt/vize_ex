defmodule Vize.SFC do
  @moduledoc """
  Bundler-oriented helpers for Vue Single File Components.

  These helpers expose Vize's SFC dependency and scope-ID logic without
  requiring callers to reimplement Vue template parsing with regular
  expressions.
  """

  alias Vize.SFC.{Asset, ExternalSource}

  @doc """
  Collect importable static asset references from an SFC template.

  By default Vize recognizes assets such as `img[src]`, `video[src]`,
  `video[poster]`, `source[src]`, and SVG image/use references.

  ## Options

    * `:filename` — filename used when parsing the SFC
  """
  @spec collect_template_assets(String.t(), keyword()) ::
          {:ok, [Asset.t()]} | {:error, Vize.Error.t()}
  def collect_template_assets(source, opts \\ []) do
    filename = Keyword.get(opts, :filename, "")

    case Vize.parse_sfc(source) do
      {:ok, _descriptor} ->
        assets =
          source
          |> Vize.Native.sfc_template_assets_nif(filename)
          |> Enum.map(&Asset.new/1)

        {:ok, assets}

      {:error, reason} ->
        {:error, Vize.Error.new("Vize SFC asset collection error", reason)}
    end
  end

  @doc "Like `collect_template_assets/2` but raises `Vize.Error` on parse errors."
  @spec collect_template_assets!(String.t(), keyword()) :: [Asset.t()]
  def collect_template_assets!(source, opts \\ []) do
    case collect_template_assets(source, opts) do
      {:ok, assets} -> assets
      {:error, error} -> raise error
    end
  end

  @doc """
  Rewrite asset literals in compiled SFC JavaScript to imported bindings.

  `assets` should be the result of `collect_template_assets/2`. The function is
  total: JavaScript that cannot be parsed is returned unchanged.
  """
  @spec rewrite_asset_references(String.t(), [Asset.t()]) :: String.t()
  def rewrite_asset_references(code, assets) when is_binary(code) and is_list(assets) do
    encoded_assets = Enum.map(assets, fn %Asset{} = asset -> {asset.url, asset.binding} end)
    Vize.Native.rewrite_sfc_template_assets_nif(code, encoded_assets)
  end

  @doc """
  Collect SFC blocks whose contents are loaded through a `src` attribute.

  Returns template, script, script-setup, style, and custom-block sources.
  """
  @spec external_sources(String.t()) ::
          {:ok, [ExternalSource.t()]} | {:error, Vize.Error.t()}
  def external_sources(source) do
    case Vize.parse_sfc(source) do
      {:ok, descriptor} -> {:ok, descriptor_external_sources(descriptor)}
      {:error, reason} -> {:error, Vize.Error.new("Vize SFC source extraction error", reason)}
    end
  end

  @doc "Like `external_sources/1` but raises `Vize.Error` on parse errors."
  @spec external_sources!(String.t()) :: [ExternalSource.t()]
  def external_sources!(source) do
    case external_sources(source) do
      {:ok, sources} -> sources
      {:error, error} -> raise error
    end
  end

  @doc """
  Generate the deterministic scope ID used by Vize's bundler integrations.

  ## Options

    * `:root` — project root used to normalize the filename
    * `:production` — use the production hashing strategy
    * `:source` — source content included by the production strategy
  """
  @spec scope_id(String.t(), keyword()) :: String.t()
  def scope_id(filename, opts \\ []) do
    root = Keyword.get(opts, :root, "")
    production = Keyword.get(opts, :production, false)
    source = Keyword.get(opts, :source, "")
    Vize.Native.sfc_scope_id_nif(filename, root, production, source)
  end

  defp descriptor_external_sources(descriptor) do
    block_sources = [
      external_source(descriptor.template, :template),
      external_source(descriptor.script, :script),
      external_source(descriptor.script_setup, :script_setup)
    ]

    style_sources =
      descriptor.styles
      |> Enum.with_index()
      |> Enum.map(fn {block, index} -> external_source(block, :style, index) end)

    custom_sources =
      descriptor.custom_blocks
      |> Enum.with_index()
      |> Enum.map(fn {block, index} ->
        external_source(block, :custom, index, block.block_type)
      end)

    Enum.reject(block_sources ++ style_sources ++ custom_sources, &is_nil/1)
  end

  defp external_source(block, type, index \\ nil, block_type \\ nil)
  defp external_source(nil, _type, _index, _block_type), do: nil
  defp external_source(%{src: nil}, _type, _index, _block_type), do: nil

  defp external_source(%{src: src}, type, index, block_type) do
    %ExternalSource{type: type, src: src, index: index, block_type: block_type}
  end
end
