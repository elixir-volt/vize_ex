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

  defp rewrite_css_url(%{"url" => from} = node, from, to), do: %{node | "url" => to}
  defp rewrite_css_url(node, _from, _to), do: node

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

    test "returns external sources and descriptor metadata" do
      source = """
      <template src="./template.html"></template>
      <script src="./component.js"></script>
      <style src="./component.css" scoped></style>
      """

      {:ok, descriptor} = Vize.parse_sfc(source)

      assert descriptor.template.src == "./template.html"
      assert descriptor.script.src == "./component.js"
      assert [%{src: "./component.css", scoped: true}] = descriptor.styles
      assert descriptor.css_vars == []
      refute descriptor.slotted
      refute descriptor.should_force_reload
    end
  end

  describe "parse_sfc!/1" do
    test "returns descriptor on success" do
      descriptor = Vize.parse_sfc!(@simple_sfc)
      assert descriptor.template.content =~ "Hello World"
    end
  end

  describe "analyze_sfc/2" do
    test "returns a Croquis summary" do
      source =
        ~S[<template><MyButton :label="msg" @click="save" /></template><script setup>const msg = "hi"; function save(){}</script>]

      assert {:ok, %Vize.Croquis{} = croquis} = Vize.analyze_sfc(source)
      assert "MyButton" in croquis.used_components

      assert [%{name: "MyButton", props: [%{name: "label"}], events: [%{name: "click"}]}] =
               croquis.component_usages
    end

    test "bang variant returns a Croquis" do
      assert %Vize.Croquis{} = Vize.analyze_sfc!("<template><div /></template>")
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

    test "accepts filename option" do
      {:ok, result} = Vize.compile_sfc(@styled_sfc, filename: "App.vue")
      assert result.errors == []
    end

    test "returns template hash" do
      {:ok, result} = Vize.compile_sfc(@simple_sfc)
      assert is_binary(result.template_hash)
    end

    test "returns style hash" do
      {:ok, result} = Vize.compile_sfc(@styled_sfc)
      assert is_binary(result.style_hash)
    end

    test "returns nil hashes for missing blocks" do
      {:ok, result} = Vize.compile_sfc("<template><div>hi</div></template>")
      assert result.template_hash != nil
      assert result.style_hash == nil
    end

    test "returns source maps for the final emitted JavaScript" do
      {:ok, result} =
        Vize.compile_sfc(@setup_sfc,
          filename: "src/Counter.vue",
          strip_types: true,
          source_map: true
        )

      assert is_binary(result.map)
      assert result.map =~ ~s("version":3)
      assert result.map =~ "src/Counter.vue"
    end

    test "returns bundler block metadata" do
      source = """
      <template><div class="container">Styled</div></template>
      <style src="./base.css"></style>
      <style scoped module="theme">.container { color: blue; }</style>
      <docs src="./readme.md">fallback</docs>
      """

      {:ok, result} = Vize.compile_sfc(source, filename: "App.vue")

      assert result.has_scoped

      assert [
               %{src: "./base.css", scoped: false, module: nil},
               %{src: nil, scoped: true, module: "theme"}
             ] = result.styles

      assert [%{block_type: "docs", src: "./readme.md", content: "fallback"}] =
               result.custom_blocks
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

    test "can return structured diagnostics" do
      {:ok, result} = Vize.compile_vapor(~s(<div id="a" id="b">x</div>), diagnostics: true)

      assert %Vize.Vapor.Result{} = result

      assert [%Vize.Diagnostic{code: "DuplicateAttribute", recoverable?: true}] =
               result.diagnostics
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

  describe "Vize.CSS URL helpers" do
    test "selects parser-backed URL events" do
      css = ".foo { background: url('./logo.svg') }"

      assert {:ok, [%{url: "./logo.svg", start: start, end: finish}]} =
               Vize.CSS.select(css, :urls)

      assert binary_part(css, start, finish - start) == "./logo.svg"
    end

    test "collects parser-backed URL ranges" do
      css = ".foo { background: url('./logo.svg') }"

      assert {:ok, [%Vize.CSS.URL{url: "./logo.svg", range: range}]} =
               Vize.CSS.collect_urls(css)

      assert binary_part(css, range.start, range.end - range.start) == "./logo.svg"
    end

    test "selects parser-backed import events" do
      css = "@import './reset.css';\n@import './print.css' print;\n.app { color: red }"

      assert {:ok,
              [
                %{
                  url: "./reset.css",
                  start: reset_start,
                  end: reset_end,
                  media: nil,
                  supports: nil
                },
                %{
                  url: "./print.css",
                  start: print_start,
                  end: print_end,
                  media: "print",
                  supports: nil
                }
              ]} = Vize.CSS.select(css, :imports)

      assert binary_part(css, reset_start, reset_end - reset_start) == "./reset.css"
      assert binary_part(css, print_start, print_end - print_start) == "./print.css"
    end

    test "selects mixed CSS dependency events" do
      css =
        "@import './theme.css' supports(display: grid);\n.logo { background: url('./logo.svg') }"

      assert {:ok,
              [
                %{kind: :import, url: "./theme.css", supports: "(display: grid)"},
                %{kind: :url, url: "./logo.svg"}
              ]} = Vize.CSS.select(css, :dependencies)
    end

    test "rewrites URLs without CSS AST print roundtrip" do
      css = ".x{left:calc(var(--vscode-sash-size)*-.5);background:url('./logo.svg')}"

      assert {:ok, rewritten} =
               Vize.CSS.rewrite_urls(css, fn
                 "./logo.svg" -> {:rewrite, "/assets/logo-hash.svg"}
                 _url -> :keep
               end)

      assert rewritten =~ "calc(var(--vscode-sash-size)*-.5)"
      assert rewritten =~ "url('/assets/logo-hash.svg')"
    end

    test "rewrites font URLs without CSS AST print roundtrip" do
      css = "@font-face { src: url(foo.ttf); }"

      assert {:ok, rewritten} =
               Vize.CSS.rewrite_urls(css, fn
                 "foo.ttf" -> {:rewrite, "/assets/foo.ttf"}
                 _url -> :keep
               end)

      assert rewritten == "@font-face { src: url(/assets/foo.ttf); }"
    end

    test "bang variants return values" do
      css = ".foo { background: url('./logo.svg') }"

      assert [%Vize.CSS.URL{url: "./logo.svg"}] = Vize.CSS.collect_urls!(css)

      assert Vize.CSS.rewrite_urls!(css, fn
               "./logo.svg" -> {:rewrite, "/assets/logo.svg"}
               _url -> :keep
             end) =~ "/assets/logo.svg"
    end
  end

  describe "Vize.CSS AST helpers" do
    test "round-trips CSS through an Elixir AST" do
      {:ok, parsed} = Vize.CSS.parse_ast(".foo { color: red }")

      assert is_map(parsed.ast)
      assert parsed.errors == []

      {:ok, printed} = Vize.CSS.print_ast(parsed.ast)

      assert printed.code =~ "color"
      assert printed.errors == []
    end

    test "supports parser-backed URL mutation" do
      {:ok, parsed} = Vize.CSS.parse_ast(".foo { background: url('./logo.svg') }")

      ast =
        Vize.CSS.postwalk(parsed.ast, &rewrite_css_url(&1, "./logo.svg", "/assets/logo-hash.svg"))

      {:ok, printed} = Vize.CSS.print_ast(ast)

      assert printed.code =~ "/assets/logo-hash.svg"
    end

    test "prints image-set ASTs" do
      css = ".hero { background-image: image-set(url('./hero.avif') type('image/avif') 1x) }"
      {:ok, parsed} = Vize.CSS.parse_ast(css)
      {:ok, printed} = Vize.CSS.print_ast(parsed.ast)

      assert printed.code =~ "image-set"
      assert printed.code =~ "hero.avif"
    end

    test "collects URL nodes" do
      {:ok, parsed} = Vize.CSS.parse_ast(".foo { background: url('./logo.svg') }")

      urls =
        Vize.CSS.collect(parsed.ast, fn
          %{"url" => url} -> {:keep, url}
          _ -> :skip
        end)

      assert "./logo.svg" in urls
    end
  end

  describe "compile_css/2" do
    test "compiles basic CSS" do
      {:ok, result} = Vize.CSS.compile(".foo { color: red }")
      assert result.code =~ "color"
      assert result.errors == []
      assert result.warnings == []
    end

    test "minifies CSS" do
      {:ok, result} = Vize.CSS.compile(".foo {\n  color: red;\n}", minify: true)
      refute result.code =~ "\n"
    end

    test "returns empty css_vars for plain CSS" do
      {:ok, result} = Vize.CSS.compile(".foo { color: red }")
      assert result.css_vars == []
    end

    test "extracts v-bind expressions" do
      {:ok, result} = Vize.CSS.compile(".foo { color: v-bind(textColor) }")
      assert "textColor" in result.css_vars
    end

    test "applies scoped transformation" do
      {:ok, result} =
        Vize.CSS.compile(".foo { color: red }", scoped: true, scope_id: "data-v-abc123")

      assert result.code =~ "abc123"
    end

    test "handles parse errors gracefully" do
      {:ok, result} = Vize.CSS.compile(".foo { color: }")
      assert length(result.errors) > 0 or result.code != ""
    end

    test "bang variant works" do
      result = Vize.CSS.compile!(".foo { color: red }")
      assert result.code =~ "color"
    end
  end

  describe "compile_sass/2" do
    test "compiles SCSS variables and nesting" do
      {:ok, result} =
        Vize.CSS.compile_sass("$color: #c00; .button { color: $color; &:hover { color: blue; } }")

      assert result.code =~ ".button"
      assert result.code =~ ".button:hover"
      assert result.code =~ "#c00"
    end

    test "compiles indented Sass syntax" do
      {:ok, result} =
        Vize.CSS.compile_sass("$color: red\n.button\n  color: $color", syntax: :sass)

      assert result.code =~ ".button"
      assert result.code =~ "color: red"
    end

    test "resolves imports relative to the filename" do
      directory = Path.join(System.tmp_dir!(), "vize-sass-#{System.unique_integer([:positive])}")
      File.mkdir_p!(directory)
      on_exit(fn -> File.rm_rf(directory) end)
      File.write!(Path.join(directory, "_colors.scss"), "$brand: rebeccapurple;")

      assert {:ok, result} =
               Vize.CSS.compile_sass("@use 'colors' as *; .logo { color: $brand; }",
                 filename: Path.join(directory, "app.scss")
               )

      assert result.code =~ "rebeccapurple"
    end

    test "returns compilation errors and bang variant raises" do
      assert {:error, error} = Vize.CSS.compile_sass(".broken { color: $missing; }")
      assert error =~ "Undefined variable"

      assert_raise RuntimeError, ~r/Vize Sass compile error/, fn ->
        Vize.CSS.compile_sass!(".broken { color: $missing; }")
      end
    end
  end

  describe "compile_sfc/2 new options" do
    test "returns macro_artifacts" do
      {:ok, result} = Vize.compile_sfc(@setup_sfc)
      assert is_list(result.macro_artifacts)
    end

    test "strip_types produces valid JavaScript" do
      ts_sfc = """
      <template><div>{{ msg }}</div></template>
      <script setup lang="ts">
      interface Props { title: string }
      const msg: string = 'hello'
      </script>
      """

      {:ok, result} = Vize.compile_sfc(ts_sfc, strip_types: true)

      refute result.code =~ "interface Props"
      refute result.code =~ ": string"
      assert result.code =~ "msg"
    end

    test "custom_renderer is accepted" do
      {:ok, result} = Vize.compile_sfc(@simple_sfc, custom_renderer: true)
      assert result.errors == []
    end
  end

  describe "generate_dts/2" do
    test "generates declaration from script setup" do
      sfc = """
      <script setup lang="ts">
      const msg = 'hello'
      </script>
      """

      {:ok, result} = Vize.generate_dts(sfc)
      assert is_binary(result.dts)
    end

    test "handles template-only SFC" do
      {:ok, result} = Vize.generate_dts("<template><div>hi</div></template>")
      assert is_binary(result.dts)
    end

    test "accepts filename option" do
      {:ok, result} = Vize.generate_dts("<script setup>const x = 1</script>", filename: "App.vue")
      assert is_binary(result.dts)
    end

    test "bang variant works" do
      result = Vize.generate_dts!("<script setup>const x = 1</script>")
      assert is_binary(result.dts)
    end
  end
end
