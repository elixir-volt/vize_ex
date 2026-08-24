defmodule Vize.SFC do
  @moduledoc """
  Bundler-oriented helpers for Vue Single File Components.

  These helpers expose Vize's SFC dependency and scope-ID logic without
  requiring callers to reimplement Vue template parsing with regular
  expressions.
  """

  @type template_asset :: %{url: String.t(), var_name: String.t()}
  @type src_info :: %{script_src: String.t() | nil, template_src: String.t() | nil}

  @doc """
  Collect importable static asset URLs from an SFC template.

  By default Vize recognizes assets such as `img[src]`, `video[src]`,
  `video[poster]`, `source[src]`, and SVG image/use references.
  """
  @spec template_assets(String.t(), keyword()) :: [template_asset()]
  def template_assets(source, opts \\ []) do
    filename = Keyword.get(opts, :filename, "")
    Vize.Native.sfc_template_assets_nif(source, filename)
  end

  @doc """
  Rewrite compiled template asset literals to their imported variable names.

  `assets` should be the result of `template_assets/2`.
  """
  @spec rewrite_template_assets(String.t(), [template_asset()]) :: String.t()
  def rewrite_template_assets(code, assets) do
    encoded_assets = Enum.map(assets, &{&1.url, &1.var_name})
    Vize.Native.rewrite_sfc_template_assets_nif(code, encoded_assets)
  end

  @doc """
  Return external `<script src>` and `<template src>` references from an SFC.
  """
  @spec src_info(String.t(), keyword()) :: src_info()
  def src_info(source, opts \\ []) do
    filename = Keyword.get(opts, :filename, "")
    Vize.Native.sfc_src_info_nif(source, filename)
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
end
