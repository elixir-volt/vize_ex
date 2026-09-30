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

  test "replaces the whole text node for mixed static and dynamic text" do
    {:ok, split} = Vize.vapor_split("<span>Hello {{ a }}, you have {{ n }} items</span>")

    assert split.statics == ["<span>", "</span>"]

    assert [
             %{
               kind: :set_text,
               values: [
                 {:static_, "Hello "},
                 "a",
                 {:static_, ", you have "},
                 "n",
                 {:static_, " items"}
               ]
             }
           ] =
             split.slots
  end

  test "matches HTML-escaped static text" do
    {:ok, split} = Vize.vapor_split("<p>\"Tom\" & {{ x }} <3</p>")

    assert split.statics == ["<p>", "</p>"]
  end

  test "places text slots after sibling elements" do
    {:ok, split} = Vize.vapor_split("<div><b>bold</b> hi {{ x }}</div>")

    assert split.statics == ["<div><b>bold</b>", "</div>"]
  end

  test "replaces Vapor anchor comments with structural slots" do
    {:ok, split} =
      Vize.vapor_split(
        "<div><p v-if=\"a\">A</p><p v-if=\"b\">B</p><i :id=\"k\">{{ x }}</i></div>"
      )

    assert Enum.map(split.slots, & &1.kind) == [:if_node, :if_node, :set_prop, :set_text]
    assert split.statics == ["<div>", "", "<i id=\"", "\">", "</i></div>"]
  end

  test "matches text nodes by decoded content, not by a substring" do
    {:ok, split} = Vize.vapor_split("<div>static <b>x</b> hi {{ a }}</div>")
    assert split.statics == ["<div>static <b>x</b>", "</div>"]

    {:ok, split} = Vize.vapor_split("<div>x {{ a }}<b>x </b>x {{ c }}</div>")
    assert split.statics == ["<div>", "<b>x </b>", "</div>"]
  end

  # https://github.com/elixir-volt/vize_ex/issues/3
  test "orders slots by document position across nesting and structural nodes" do
    {:ok, split} = Vize.vapor_split(~s(<a :class="l1"><b :class="l2"><c :class="l3"></c></b></a>))
    assert Enum.map(split.slots, & &1.values) == [["l1"], ["l2"], ["l3"]]

    {:ok, split} = Vize.vapor_split(~s(<div><b v-if="on">Y</b><span>{{ label }}</span></div>))
    assert Enum.map(split.slots, & &1.kind) == [:if_node, :set_text]
    assert split.statics == ["<div>", "<span>", "</span></div>"]

    {:ok, split} =
      Vize.vapor_split(
        ~s(<div><section><b v-if="on">Y</b></section><span>{{ label }}</span></div>)
      )

    assert Enum.map(split.slots, & &1.kind) == [:if_node, :set_text]
    assert split.statics == ["<div><section>", "</section><span>", "</span></div>"]
  end

  test "places v-html slots inside their element" do
    {:ok, split} = Vize.vapor_split(~s(<div v-html="raw"></div>))
    assert split.statics == ["<div>", "</div>"]
    assert [%{kind: :set_html, value: "raw"}] = split.slots

    {:ok, split} = Vize.vapor_split(~s(<section><p v-html="raw"></p><b>{{ x }}</b></section>))
    assert split.statics == ["<section><p>", "</p><b>", "</b></section>"]
    assert Enum.map(split.slots, & &1.kind) == [:set_html, :set_text]
  end
end
