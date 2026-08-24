defmodule Vize.Integration.CSSTest do
  use ExUnit.Case, async: false

  @moduletag :integration

  defp tmp_dir do
    dir = Path.join(System.tmp_dir!(), "vize-ex-#{System.unique_integer([:positive])}")
    File.mkdir_p!(dir)
    on_exit(fn -> File.rm_rf!(dir) end)
    dir
  end

  test "bundle/2 resolves and inlines imported files" do
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

  test "compile_sass/2 resolves imports relative to the filename" do
    dir = tmp_dir()
    File.write!(Path.join(dir, "_colors.scss"), "$brand: rebeccapurple;")

    assert {:ok, result} =
             Vize.CSS.compile_sass("@use 'colors' as *; .logo { color: $brand; }",
               filename: Path.join(dir, "app.scss")
             )

    assert result.code =~ "rebeccapurple"
  end
end
