#!/usr/bin/env bash
# Print the Homebrew formula for a released version, fetching the asset hashes
# from the GitHub release. Usage: scripts/homebrew-formula.sh 0.1.0 > Formula/conveyor.rb
set -euo pipefail

version="${1:?version without the leading v}"
base="https://github.com/piacsek/conveyor/releases/download/v${version}"

sha() {
  curl -fsSL "${base}/conveyor-$1.tar.gz.sha256" | cut -d' ' -f1
}

arm_mac="$(sha aarch64-apple-darwin)"
intel_mac="$(sha x86_64-apple-darwin)"
linux="$(sha x86_64-unknown-linux-gnu)"

cat <<FORMULA
class Conveyor < Formula
  desc "TUI for the path of a change: pull requests, merge queue, main builds, deployed"
  homepage "https://github.com/piacsek/conveyor"
  license "MIT"

  depends_on "gh"

  on_macos do
    on_arm do
      url "${base}/conveyor-aarch64-apple-darwin.tar.gz"
      sha256 "${arm_mac}"
    end
    on_intel do
      url "${base}/conveyor-x86_64-apple-darwin.tar.gz"
      sha256 "${intel_mac}"
    end
  end

  on_linux do
    on_intel do
      url "${base}/conveyor-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "${linux}"
    end
  end

  def install
    bin.install "conveyor"
  end

  test do
    assert_match "usage", shell_output("#{bin}/conveyor bogus 2>&1", 1)
    assert_match "[prs]", shell_output("#{bin}/conveyor config")
  end
end
FORMULA
