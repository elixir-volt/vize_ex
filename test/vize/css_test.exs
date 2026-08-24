defmodule Vize.CSSTest do
  use ExUnit.Case, async: true

  defp rewrite_css_url(%{"url" => from} = node, from, to), do: %{node | "url" => to}
  defp rewrite_css_url(node, _from, _to), do: node

  describe "select/3 and URL helpers" do
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

  describe "AST helpers" do
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

  describe "compile/2" do
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

    test "returns CSS Modules exports" do
      {:ok, result} =
        Vize.CSS.compile(".btn { color: red }",
          css_modules: true,
          filename: "button.module.css"
        )

      assert result.errors == []
      assert is_map(result.exports)
      assert is_binary(result.exports["btn"])
      assert result.exports["btn"] != "btn"
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

    test "returns compilation errors and bang variant raises" do
      assert {:error, error} = Vize.CSS.compile_sass(".broken { color: $missing; }")
      assert error =~ "Undefined variable"

      assert_raise RuntimeError, ~r/Vize Sass compile error/, fn ->
        Vize.CSS.compile_sass!(".broken { color: $missing; }")
      end
    end
  end
end
