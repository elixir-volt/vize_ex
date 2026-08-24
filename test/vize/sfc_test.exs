defmodule Vize.SFCTest do
  use ExUnit.Case, async: true

  alias Vize.SFC
  alias Vize.SFC.{Asset, ExternalSource}

  @asset_sfc """
  <template>
    <div>
      <img src="./logo.png">
      <video poster="../poster.jpg"><source src="https://example.com/movie.mp4"></video>
      <svg><use href="@/icons.svg#check" /></svg>
    </div>
  </template>
  """

  test "collect_template_assets/2 returns typed importable asset references" do
    assert {:ok,
            [
              %Asset{url: "./logo.png", binding: "_imports_0"},
              %Asset{url: "../poster.jpg", binding: "_imports_1"},
              %Asset{url: "@/icons.svg#check", binding: "_imports_2"}
            ]} = SFC.collect_template_assets(@asset_sfc, filename: "src/App.vue")
  end

  test "collect_template_assets!/2 returns assets directly" do
    assert [%Asset{} | _] = SFC.collect_template_assets!(@asset_sfc)
  end

  test "collect_template_assets/2 reports SFC parse errors" do
    invalid = "<template><div>"

    assert {:error, %Vize.Error{}} = SFC.collect_template_assets(invalid)
    assert_raise Vize.Error, fn -> SFC.collect_template_assets!(invalid) end
  end

  test "rewrite_asset_references/2 replaces compiled render literals" do
    assets = SFC.collect_template_assets!(@asset_sfc)
    compiled = Vize.compile_sfc!(@asset_sfc).code
    rewritten = SFC.rewrite_asset_references(compiled, assets)

    assert rewritten =~ "_imports_0"
    assert rewritten =~ "_imports_2"
    refute rewritten =~ ~S["./logo.png"]
    refute rewritten =~ ~S["@/icons.svg#check"]
  end

  test "rewrite_asset_references/2 leaves invalid JavaScript unchanged" do
    assets = SFC.collect_template_assets!(@asset_sfc)
    assert SFC.rewrite_asset_references("const =", assets) == "const ="
  end

  test "external_sources/1 returns every externally loaded block" do
    source = """
    <template src="./view.html"></template>
    <script src="./logic.js"></script>
    <style src="./base.css"></style>
    <style src="./theme.scss" lang="scss"></style>
    <docs src="./readme.md"></docs>
    """

    assert {:ok,
            [
              %ExternalSource{type: :template, src: "./view.html"},
              %ExternalSource{type: :script, src: "./logic.js"},
              %ExternalSource{type: :style, index: 0, src: "./base.css"},
              %ExternalSource{type: :style, index: 1, src: "./theme.scss"},
              %ExternalSource{
                type: :custom,
                index: 0,
                block_type: "docs",
                src: "./readme.md"
              }
            ]} = SFC.external_sources(source)
  end

  test "external_sources!/1 returns sources directly" do
    source = ~S[<template src="./view.html"></template>]

    assert [%ExternalSource{type: :template, src: "./view.html"}] =
             SFC.external_sources!(source)
  end

  test "external_sources/1 reports SFC parse errors" do
    invalid = "<template><div>"

    assert {:error, %Vize.Error{}} = SFC.external_sources(invalid)
    assert_raise Vize.Error, fn -> SFC.external_sources!(invalid) end
  end

  test "scope_id/2 is deterministic and normalized" do
    first = SFC.scope_id("/project/src/App.vue", root: "/project")
    second = SFC.scope_id("/project/src/App.vue", root: "/project")

    assert first == second
    assert is_binary(first)
    assert first != ""
  end
end
