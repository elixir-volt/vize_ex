defmodule Vize.MixProject do
  use Mix.Project

  @version "0.13.1"
  @source_url "https://github.com/elixir-volt/vize_ex"

  def project do
    [
      app: :vize,
      version: @version,
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      aliases: aliases(),
      dialyzer: [plt_add_apps: [:mix, :rustq], flags: [:no_opaque]],
      name: "Vize",
      description:
        "Elixir bindings for the Vize Vue.js toolchain — compile, lint, and format Vue SFCs via Rust NIFs.",
      source_url: @source_url,
      homepage_url: @source_url,
      package: package(),
      docs: docs()
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp package do
    [
      licenses: ["MIT"],
      links: %{
        "GitHub" => @source_url,
        "Vize" => "https://vizejs.dev"
      },
      files:
        ~w(lib native/vize_ex_nif/src native/vize_ex_nif/Cargo.toml Cargo.toml Cargo.lock .formatter.exs mix.exs rustq.exs README.md LICENSE checksum-*.exs)
    ]
  end

  defp docs do
    [
      main: "Vize",
      extras: ["README.md", "LICENSE"],
      source_ref: "v#{@version}"
    ]
  end

  defp aliases do
    [
      lint: [
        "rustq.gen --check",
        "format --check-formatted",
        "credo --strict",
        "ex_dna",
        "reach.check --dead-code --smells --strict",
        "dialyzer",
        "cmd cargo fmt --manifest-path native/vize_ex_nif/Cargo.toml -- --check",
        "cmd cargo clippy --manifest-path native/vize_ex_nif/Cargo.toml -- -D warnings"
      ],
      ci: ["lint", "cmd env MIX_ENV=test mix test"]
    ]
  end

  defp deps do
    [
      {:rustler, "~> 0.36 or ~> 0.37 or ~> 0.38", optional: true},
      {:rustler_precompiled, "~> 0.8"},
      {:rustq, github: "dannote/rustq", ref: "56147d4", only: [:dev, :test], runtime: false},
      {:ex_doc, "~> 0.40", only: :dev, runtime: false},
      {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false},
      {:credo, "~> 1.7", only: [:dev, :test], runtime: false},
      {:ex_dna, "~> 1.5", only: [:dev, :test], runtime: false},
      {:ex_slop, "~> 0.4", only: [:dev, :test], runtime: false},
      {:reach, "~> 2.0", only: [:dev, :test], runtime: false}
    ]
  end
end
