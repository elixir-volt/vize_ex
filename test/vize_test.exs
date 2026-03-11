defmodule VizeTest do
  use ExUnit.Case, async: true
  doctest Vize

  @simple_sfc """
  <template>
    <div>Hello World</div>
  </template>

  <script>
  export default {
    name: 'HelloWorld'
  }
  </script>
  """

  @setup_sfc """
  <template>
    <button @click="count++">{{ count }}</button>
  </template>

  <script setup>
  import { ref } from 'vue'
  const count = ref(0)
  </script>
  """

  @styled_sfc """
  <template>
    <div class="container">Styled</div>
  </template>

  <style scoped>
  .container { background: blue; }
  </style>
  """

  describe "parse_sfc/1" do
    test "parses template block" do
      {:ok, descriptor} = Vize.parse_sfc(@simple_sfc)
      assert descriptor.template.content =~ "Hello World"
    end

    test "parses script block" do
      {:ok, descriptor} = Vize.parse_sfc(@simple_sfc)
      assert descriptor.script.content =~ "HelloWorld"
      refute descriptor.script.setup
    end

    test "parses script setup" do
      {:ok, descriptor} = Vize.parse_sfc(@setup_sfc)
      assert descriptor.script_setup.content =~ "ref"
      assert descriptor.script_setup.setup
    end

    test "parses scoped style" do
      {:ok, descriptor} = Vize.parse_sfc(@styled_sfc)
      assert [style] = descriptor.styles
      assert style.scoped
      assert style.content =~ "background: blue"
    end

    test "returns nil for missing blocks" do
      {:ok, descriptor} = Vize.parse_sfc("<template><div>hi</div></template>")
      assert descriptor.template != nil
      assert descriptor.script == nil
      assert descriptor.script_setup == nil
      assert descriptor.styles == []
    end
  end

  describe "parse_sfc!/1" do
    test "returns descriptor on success" do
      descriptor = Vize.parse_sfc!(@simple_sfc)
      assert descriptor.template.content =~ "Hello World"
    end
  end

  describe "compile_sfc/2" do
    test "compiles simple SFC" do
      {:ok, result} = Vize.compile_sfc(@simple_sfc)
      assert result.code =~ "Hello World"
      assert result.errors == []
    end

    test "compiles script setup" do
      {:ok, result} = Vize.compile_sfc(@setup_sfc)
      assert result.code =~ "count"
      assert result.errors == []
    end

    test "compiles scoped styles" do
      {:ok, result} = Vize.compile_sfc(@styled_sfc)
      assert result.css != nil
      assert result.css =~ "background"
    end

    test "compiles in vapor mode" do
      {:ok, result} = Vize.compile_sfc(@setup_sfc, vapor: true)
      assert result.code =~ "count"
      assert result.errors == []
    end

    test "compiles template-only SFC" do
      {:ok, result} = Vize.compile_sfc("<template><div>{{ msg }}</div></template>")
      assert result.code =~ "msg"
    end
  end

  describe "compile_sfc!/2" do
    test "returns result on success" do
      result = Vize.compile_sfc!(@simple_sfc)
      assert result.code =~ "Hello World"
    end
  end

  describe "compile_template/2" do
    test "compiles simple template" do
      {:ok, result} = Vize.compile_template("<div>hello</div>")
      assert result.code =~ "hello"
    end

    test "compiles template with interpolation" do
      {:ok, result} = Vize.compile_template("<div>{{ msg }}</div>")
      assert result.code =~ "msg"
    end

    test "compiles template with v-if" do
      {:ok, result} = Vize.compile_template("<div v-if=\"show\">visible</div>")
      assert result.code =~ "show"
    end

    test "compiles template with v-for" do
      {:ok, result} = Vize.compile_template("<div v-for=\"item in items\">{{ item }}</div>")
      assert result.code =~ "items"
    end

    test "compiles in module mode" do
      {:ok, result} = Vize.compile_template("<div>hello</div>", mode: "module")
      assert result.code =~ "export function render"
    end
  end

  describe "compile_template!/2" do
    test "returns result on success" do
      result = Vize.compile_template!("<div>hello</div>")
      assert result.code =~ "hello"
    end
  end

  describe "compile_ssr/1" do
    test "generates SSR code with _push" do
      {:ok, result} = Vize.compile_ssr("<div>hello</div>")
      assert result.code =~ "_push"
    end

    test "uses ssrInterpolate for dynamic content" do
      {:ok, result} = Vize.compile_ssr("<div>{{ msg }}</div>")
      assert result.code =~ "ssrInterpolate" or result.code =~ "_ssrInterpolate"
    end
  end

  describe "compile_ssr!/1" do
    test "returns result on success" do
      result = Vize.compile_ssr!("<div>hello</div>")
      assert result.code =~ "_push"
    end
  end

  describe "compile_vapor/2" do
    test "compiles to vapor mode" do
      {:ok, result} = Vize.compile_vapor("<div>hello</div>")
      assert result.code =~ "template"
      assert length(result.templates) > 0
    end

    test "generates reactive effects for interpolation" do
      {:ok, result} = Vize.compile_vapor("<div>{{ msg }}</div>")
      assert result.code =~ "renderEffect" or result.code =~ "setText"
    end

    test "handles v-if" do
      {:ok, result} = Vize.compile_vapor("<div v-if=\"show\">visible</div>")
      assert result.code =~ "createIf"
    end

    test "handles v-for" do
      {:ok, result} = Vize.compile_vapor("<div v-for=\"item in items\">{{ item }}</div>")
      assert result.code =~ "createFor"
    end

    test "handles events" do
      {:ok, result} = Vize.compile_vapor("<button @click=\"onClick\">click</button>")
      assert result.code =~ "click"
    end
  end

  describe "compile_vapor!/2" do
    test "returns result on success" do
      result = Vize.compile_vapor!("<div>hello</div>")
      assert is_binary(result.code)
    end
  end

  describe "vapor_ir/1" do
    test "returns IR with templates" do
      {:ok, ir} = Vize.vapor_ir("<div>hello</div>")
      assert length(ir.templates) > 0
      assert hd(ir.templates) =~ "<div>"
    end

    test "returns block with operations" do
      {:ok, ir} = Vize.vapor_ir("<div :class=\"cls\">text</div>")
      assert is_map(ir.block)
      assert is_list(ir.block.operations)
      assert is_list(ir.block.effects)
      assert is_list(ir.block.returns)
    end

    test "captures set_text for interpolation" do
      {:ok, ir} = Vize.vapor_ir("<div>{{ msg }}</div>")

      all_ops =
        ir.block.effects
        |> List.flatten()
        |> Enum.filter(&is_map/1)

      assert Enum.any?(all_ops, &(&1[:kind] == :set_text))
    end

    test "captures if_node for v-if" do
      {:ok, ir} = Vize.vapor_ir("<div v-if=\"show\">visible</div>")
      assert Enum.any?(ir.block.operations, &(&1[:kind] == :if_node))
    end

    test "captures for_node for v-for" do
      {:ok, ir} = Vize.vapor_ir("<div v-for=\"item in items\">{{ item }}</div>")
      assert Enum.any?(ir.block.operations, &(&1[:kind] == :for_node))
    end

    test "captures component names" do
      {:ok, ir} = Vize.vapor_ir("<MyComponent />")
      assert "MyComponent" in ir.components or length(ir.block.operations) > 0
    end

    test "captures set_prop for dynamic binding" do
      {:ok, ir} = Vize.vapor_ir("<div :class=\"cls\">x</div>")

      all_ops =
        (ir.block.operations ++ List.flatten(ir.block.effects))
        |> Enum.filter(&is_map/1)

      has_set_prop = Enum.any?(all_ops, &(&1[:kind] == :set_prop))

      has_set_class =
        Enum.any?(all_ops, fn op -> op[:kind] in [:set_prop, :set_dynamic_props] end)

      assert has_set_prop or has_set_class
    end

    test "captures set_event for event binding" do
      {:ok, ir} = Vize.vapor_ir("<button @click=\"onClick\">x</button>")
      assert Enum.any?(ir.block.operations, &(&1[:kind] == :set_event))
    end
  end

  describe "vapor_ir!/1" do
    test "returns IR on success" do
      ir = Vize.vapor_ir!("<div>hello</div>")
      assert is_list(ir.templates)
    end

    test "returns empty IR for empty input" do
      ir = Vize.vapor_ir!("")
      assert ir.templates == []
      assert ir.block.operations == []
    end
  end

  describe "lint/2" do
    test "returns diagnostics list" do
      {:ok, diagnostics} = Vize.lint("<template><div>ok</div></template>", "test.vue")
      assert is_list(diagnostics)
    end
  end
end
