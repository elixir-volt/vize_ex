defmodule VizeIntegrationTest do
  use ExUnit.Case, async: false

  defp tmp_dir do
    dir = Path.join(System.tmp_dir!(), "vize-ex-#{System.unique_integer([:positive])}")
    File.mkdir_p!(dir)
    on_exit(fn -> File.rm_rf!(dir) end)
    dir
  end

  test "bundle_css/2 inlines imported files" do
    dir = tmp_dir()
    child_path = Path.join(dir, "child.css")
    entry_path = Path.join(dir, "app.css")

    File.write!(child_path, ".child { color: blue; }")
    File.write!(entry_path, "@import \"./child.css\";\n.root { color: red; }")

    {:ok, result} = Vize.CSS.bundle(entry_path)

    assert result.errors == []
    assert result.code =~ ".child"
    assert result.code =~ ".root"
    refute result.code =~ "@import"
  end

  test "compile_css/2 returns CSS Modules exports" do
    {:ok, result} =
      Vize.CSS.compile(".btn { color: red }", css_modules: true, filename: "button.module.css")

    assert result.errors == []
    assert is_map(result.exports)
    assert is_binary(result.exports["btn"])
    assert result.exports["btn"] != "btn"
  end

  test "vapor_split/1 preserves self-closing tag syntax when injecting attrs" do
    {:ok, split} = Vize.vapor_split("<div><input v-model=\"name\" /></div>")

    statics = Enum.join(split.statics)

    assert statics =~ "phx-change=\"name_changed\""
    assert statics =~ "<input"
    assert statics =~ "value=\"\""
    refute statics =~ "/ phx-change"
    refute statics =~ "/ value"
    assert Enum.any?(split.slots, &(&1.kind == :v_model))
  end

  test "vapor_split/1 handles sibling roots" do
    {:ok, split} = Vize.vapor_split("<div>{{ one }}</div><span>{{ two }}</span>")

    assert length(split.statics) >= 3
    assert Enum.count(split.slots, &(&1.kind == :set_text)) == 2
  end

  test "vapor_split/1 keeps slot ordering aligned with static markers" do
    {:ok, split} =
      Vize.vapor_split("<div :class=\"cls\">{{ msg }}</div><div v-if=\"show\">ok</div>")

    assert Enum.map(split.slots, & &1.kind) == [:set_prop, :set_text, :if_node]
    assert length(split.statics) == length(split.slots) + 1
  end
end
