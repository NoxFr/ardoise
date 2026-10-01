#!/bin/sh
# Usage : homebrew-formula.sh <version> <dossier des archives>  →  formule Homebrew sur stdout
set -eu

version=$1
dir=$2
base="https://github.com/noxfr/ardoise/releases/download/v$version"
sha() { sha256sum "$dir/ardoise-$1.tar.gz" | cut -d' ' -f1; }

cat <<EOF
class Ardoise < Formula
  desc "Widget de bureau : consommation de Claude Code, Codex et OpenCode"
  homepage "https://github.com/noxfr/ardoise"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "$base/ardoise-macos-arm64.tar.gz"
      sha256 "$(sha macos-arm64)"
    end
    on_intel do
      url "$base/ardoise-macos-x86_64.tar.gz"
      sha256 "$(sha macos-x86_64)"
    end
  end

  on_linux do
    on_intel do
      url "$base/ardoise-linux-x86_64.tar.gz"
      sha256 "$(sha linux-x86_64)"
    end
  end

  def install
    bin.install "ardoise"
  end

  test do
    assert_predicate bin/"ardoise", :executable?
  end
end
EOF
