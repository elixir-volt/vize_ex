defmodule Vize.SplitTemplateTest do
  use ExUnit.Case, async: true

  defp split(template, opts \\ []), do: Vize.split_template!(template, opts)
  defp kinds(%{slots: slots}), do: Enum.map(slots, & &1.kind)

  describe "static content" do
    test "is one static" do
      assert split(~S|<div class="a"><p>Hello</p><br></div>|).statics ==
               [~S|<div class="a"><p>Hello</p><br></div>|]
    end

    test "decodes entities once and escapes them again" do
      assert split(~S|<p title="a &amp; b &lt;c&gt;">x &lt; y &amp;&amp; z</p>|).statics ==
               [~S|<p title="a &amp; b &lt;c&gt;">x &lt; y &amp;&amp; z</p>|]
    end

    test "closes self-closing elements that aren't void" do
      assert split(~S|<div><span /><input /></div>|).statics == [
               "<div><span></span><input></div>"
             ]
    end
  end

  describe "text" do
    test "an interpolation is a text slot" do
      result = split(~S|<p>{{ name }}</p>|)

      assert result.statics == ["<p>", "</p>"]
      assert [%{kind: :text, value: "name", position: {1, 7}}] = result.slots
    end

    test "static text around interpolations stays in the statics" do
      result = split(~S|<p>Hi {{ first }} {{ last }}!</p>|)

      assert result.statics == ["<p>Hi ", " ", "!</p>"]
      assert Enum.map(result.slots, & &1.value) == ["first", "last"]
    end

    test "v-html and v-text replace the element's content" do
      result = split(~S|<div v-html="raw"></div><p v-text="msg"></p>|)

      assert result.statics == ["<div>", "</div><p>", "</p>"]
      assert kinds(result) == [:html, :text]
    end
  end

  describe "attributes" do
    test "a binding is an attribute slot, rendered whole" do
      result = split(~S|<a :href="url" title="t">x</a>|)

      assert result.statics == [~S|<a title="t"|, ">x</a>"]
      assert [%{kind: :attr, name: "href", value: "url", static: nil}] = result.slots
    end

    test "takes in a static attribute of the same name" do
      result = split(~S|<p class="a" :class="{ on: active }">x</p>|)

      assert result.statics == ["<p", ">x</p>"]
      assert [%{name: "class", static: "a", value: "{ on: active }"}] = result.slots
    end

    test "v-show combines with the style" do
      assert [%{name: "style", static: "color: red", value: nil, show: "visible"}] =
               split(~S|<p style="color: red" v-show="visible">x</p>|).slots
    end

    test "dynamic names and v-bind objects" do
      result = split(~S|<p :[name]="v" v-bind="attrs">x</p>|)

      assert [
               %{kind: :attr, name: nil, name_value: "name", value: "v"},
               %{kind: :spread, value: "attrs"}
             ] =
               result.slots
    end

    test ":key isn't rendered" do
      assert split(~S|<p :key="id">x</p>|).slots == []
    end
  end

  describe "bindings" do
    test "events are reported where the element's start tag ends" do
      result = split(~S|<button class="b" @click.prevent="save">Save</button>|)

      assert [%{kind: :on, name: "click", modifiers: ["prevent"], value: "save", at: {0, offset}}] =
               result.bindings

      assert binary_part(hd(result.statics), 0, offset) == ~S|<button class="b"|
    end

    test "v-model on an input is a binding and a slot for its value" do
      result = split(~S|<input type="checkbox" value="a" v-model="picked">|)

      assert [%{kind: :model, value: "picked"}] = result.bindings

      assert [%{kind: :model, tag: "input", type: "checkbox", static_value: "a", value: "picked"}] =
               result.slots
    end

    test "v-model on a textarea renders its content" do
      result = split(~S|<textarea v-model="body"></textarea>|)

      assert result.statics == ["<textarea>", "</textarea>"]
      assert [%{kind: :model, tag: "textarea"}] = result.slots
    end
  end

  describe "structure" do
    test "v-if branches are blocks" do
      result =
        split(~S|<div><p v-if="a">A {{ x }}</p><p v-else-if="b">B</p><p v-else>C</p></div>|)

      assert [%{kind: :if, branches: [first, second, third]}] = result.slots
      assert %{condition: "a", block: %{statics: ["<p>A ", "</p>"]}} = first
      assert %{condition: "b", block: %{statics: ["<p>B</p>"]}} = second
      assert %{condition: nil, block: %{statics: ["<p>C</p>"]}} = third
    end

    test "v-for has its source, aliases, key, and block" do
      result =
        split(~S|<ul><li v-for="(item, i) in items" :key="item.id">{{ item.name }}</li></ul>|)

      assert [
               %{
                 kind: :for,
                 source: "items",
                 value: "item",
                 key: "i",
                 key_prop: "item.id",
                 block: %{statics: ["<li>", "</li>"]}
               }
             ] = result.slots
    end

    test "template wrappers render only their content" do
      result = split(~S|<template v-if="a"><b>1</b>2</template>|)

      assert [%{branches: [%{block: %{statics: ["<b>1</b>2"]}}]}] = result.slots
    end

    test "slots keep their place among static siblings" do
      result = split(~S|<div><b>x</b><Card /><i>{{ y }}</i><Card /></div>|)

      assert result.statics == ["<div><b>x</b>", "<i>", "</i>", "</div>"]
      assert kinds(result) == [:component, :text, :component]
    end
  end

  describe "components" do
    test "have props, events, and slot content" do
      result =
        split(
          ~S|<Card title="T" :count="n" @close="shut"><p>{{ x }}</p><template #footer="{ ok }">F {{ ok }}</template></Card>|
        )

      assert [%{kind: :component, name: "Card", props: props, events: events, slots: slots}] =
               result.slots

      assert [%{name: "title", static: "T"}, %{name: "count", value: "n"}] = props
      assert [%{name: "close", value: "shut"}] = events

      assert [
               %{name: "footer", params: "{ ok }", block: %{statics: ["F ", ""]}},
               %{name: "default", params: nil, block: %{statics: ["<p>", "</p>"]}}
             ] = slots
    end

    test "v-slot on the component makes its children the default slot" do
      assert [%{slots: [%{name: "default", params: "{ item }"}]}] =
               split(~S|<List v-slot="{ item }">{{ item }}</List>|).slots
    end

    test "v-model is a prop and an update event" do
      assert [
               %{
                 props: [%{name: "modelValue", value: "q"}],
                 events: [%{name: "update:modelValue", value: "q"}]
               }
             ] =
               split(~S|<Search v-model="q" />|).slots
    end

    test "a <slot> outlet has its name, props, and fallback" do
      result = split(~S|<footer><slot name="end" :count="n">none</slot></footer>|)

      assert [
               %{
                 kind: :slot,
                 name: "end",
                 props: [%{name: "count", value: "n"}],
                 fallback: %{statics: ["none"]}
               }
             ] = result.slots
    end
  end

  describe "root_attrs" do
    test "gathers the root element's attributes into one slot" do
      result =
        split(~S|<button type="button" class="btn" :class="c" disabled><slot /></button>|,
          root_attrs: true
        )

      assert result.statics == ["<button", ">", "</button>"]
      assert [%{kind: :root_attrs, props: props}, %{kind: :slot}] = result.slots

      assert Enum.map(props, &{&1.name, &1.static, &1.value}) == [
               {"type", "button", nil},
               {"class", "btn", "c"},
               {"disabled", "", nil}
             ]
    end

    test "leaves templates with several roots alone" do
      assert split(~S|<p class="a">x</p><p>y</p>|, root_attrs: true).statics ==
               [~S|<p class="a">x</p><p>y</p>|]
    end
  end

  describe "diagnostics" do
    test "an expression that doesn't parse is an error with its position" do
      assert {:error, %Vize.Error{diagnostics: [diagnostic]} = error} =
               Vize.split_template(~S|<p>{{ a + }}</p>|)

      assert %Vize.Diagnostic{severity: :error, location: %{start: %{line: 1, column: 7}}} =
               diagnostic

      assert Exception.message(error) =~ "1:7: can't parse the expression `a +`"
    end

    test "a custom directive is a warning" do
      assert %{diagnostics: [%Vize.Diagnostic{severity: :warning, message: message}]} =
               split(~S|<input v-focus>|)

      assert message =~ "v-focus"
    end

    test "convert to Elixir's diagnostic shape relative to a file" do
      {:error, %Vize.Error{diagnostics: [diagnostic]}} = Vize.split_template("<p>\n{{ a + }}</p>")

      assert %{file: "Page.vue", severity: :error, position: {11, 4}} =
               Vize.Diagnostic.to_code_diagnostic(diagnostic, file: "Page.vue", origin: {10, 5})
    end
  end
end
