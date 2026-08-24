defmodule Vize.VaporSplitTest do
  use ExUnit.Case, async: true

  test "preserves self-closing tag syntax when injecting attrs" do
    {:ok, split} = Vize.vapor_split("<div><input v-model=\"name\" /></div>")

    statics = Enum.join(split.statics)

    assert statics =~ "phx-change=\"name_changed\""
    assert statics =~ "<input"
    assert statics =~ "value=\"\""
    refute statics =~ "/ phx-change"
    refute statics =~ "/ value"
    assert Enum.any?(split.slots, &(&1.kind == :v_model))
  end

  test "handles sibling roots" do
    {:ok, split} = Vize.vapor_split("<div>{{ one }}</div><span>{{ two }}</span>")

    assert length(split.statics) >= 3
    assert Enum.count(split.slots, &(&1.kind == :set_text)) == 2
  end

  test "keeps slot ordering aligned with static markers" do
    {:ok, split} =
      Vize.vapor_split("<div :class=\"cls\">{{ msg }}</div><div v-if=\"show\">ok</div>")

    assert Enum.map(split.slots, & &1.kind) == [:set_prop, :set_text, :if_node]
    assert length(split.statics) == length(split.slots) + 1
  end

  test "orders nested property slots by document position" do
    {:ok, split} =
      Vize.vapor_split("<div :class=\"outer\"><i :class=\"inner\"></i></div>")

    assert [
             %{kind: :set_prop, values: ["outer"]},
             %{kind: :set_prop, values: ["inner"]}
           ] = split.slots

    assert split.statics == ["<div class=\"", "\"><i class=\"", "\"></i></div>"]
  end

  test "orders structural and text slots by document position" do
    {:ok, leading_if} =
      Vize.vapor_split("<div><p v-if=\"show\">visible</p>{{ message }}</div>")

    assert Enum.map(leading_if.slots, & &1.kind) == [:if_node, :set_text]
    assert length(leading_if.statics) == length(leading_if.slots) + 1

    {:ok, trailing_if} =
      Vize.vapor_split("<div>{{ message }}<p v-if=\"show\">visible</p></div>")

    assert Enum.map(trailing_if.slots, & &1.kind) == [:set_text, :if_node]
    assert length(trailing_if.statics) == length(trailing_if.slots) + 1
  end

  test "positions structural slots between static siblings" do
    {:ok, split} =
      Vize.vapor_split("<div><span>before</span><p v-if=\"show\">visible</p><b>after</b></div>")

    assert Enum.map(split.slots, & &1.kind) == [:if_node]
    assert split.statics == ["<div><span>before</span>", "<b>after</b></div>"]
  end

  test "distinguishes removed and retained tags with the same name" do
    {:ok, split} =
      Vize.vapor_split("<div><p v-if=\"show\">conditional</p><p>static</p></div>")

    assert Enum.map(split.slots, & &1.kind) == [:if_node]
    assert split.statics == ["<div>", "<p>static</p></div>"]
  end

  test "preserves static content between structural and text slots" do
    {:ok, split} =
      Vize.vapor_split("<div><p v-if=\"show\">visible</p><span>static</span>{{ message }}</div>")

    assert Enum.map(split.slots, & &1.kind) == [:if_node, :set_text]
    assert split.statics == ["<div>", "<span>static</span>", "</div>"]
  end

  test "positions v-for slots between static siblings" do
    {:ok, split} =
      Vize.vapor_split(
        "<ul><li>before</li><li v-for=\"item in items\">{{ item }}</li><li>after</li></ul>"
      )

    assert Enum.map(split.slots, & &1.kind) == [:for_node]
    assert split.statics == ["<ul><li>before</li>", "<li>after</li></ul>"]
  end
end
