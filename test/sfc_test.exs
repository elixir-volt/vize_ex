defmodule Vize.SFCTest do
  use ExUnit.Case, async: true

  alias Vize.SFC

  @asset_sfc """
  <template>
    <div>
      <img src="./logo.png">
      <video poster="../poster.jpg"><source src="https://example.com/movie.mp4"></video>
      <svg><use href="@/icons.svg#check" /></svg>
    </div>
  </template>
  """

  test "template_assets/2 collects importable static template assets" do
    assert [
             %{url: "./logo.png", var_name: "_imports_0"},
             %{url: "../poster.jpg", var_name: "_imports_1"},
             %{url: "@/icons.svg#check", var_name: "_imports_2"}
           ] = SFC.template_assets(@asset_sfc, filename: "src/App.vue")
  end

  test "rewrite_template_assets/2 replaces compiled render literals" do
    assets = SFC.template_assets(@asset_sfc)
    compiled = Vize.compile_sfc!(@asset_sfc).code
    rewritten = SFC.rewrite_template_assets(compiled, assets)

    assert rewritten =~ "_imports_0"
    assert rewritten =~ "_imports_2"
    refute rewritten =~ ~S["./logo.png"]
    refute rewritten =~ ~S["@/icons.svg#check"]
  end

  test "src_info/2 returns external script and template references" do
    source = """
    <template src="./view.html"></template>
    <script src="./logic.js"></script>
    """

    assert SFC.src_info(source) == %{
             script_src: "./logic.js",
             template_src: "./view.html"
           }
  end

  test "scope_id/2 is deterministic and normalized" do
    first = SFC.scope_id("/project/src/App.vue", root: "/project")
    second = SFC.scope_id("/project/src/App.vue", root: "/project")

    assert first == second
    assert is_binary(first)
    assert first != ""
  end
end
