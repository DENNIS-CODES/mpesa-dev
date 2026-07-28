# Homebrew formula template for mpesa-dev.
#
# This file is NOT auto-published — Homebrew taps live in their own repo
# (conventionally <owner>/homebrew-tap), which is outside this repository's
# scope. To publish it:
#
#   1. Cut a release (push a `vX.Y.Z` tag; see .github/workflows/release.yml)
#   2. Compute the sha256 of each release tarball:
#        shasum -a 256 mpesa-dev-x86_64-apple-darwin.tar.gz
#   3. Fill in the version and sha256 values below
#   4. Copy this file into DENNIS-CODES/homebrew-tap as Formula/mpesa-dev.rb
#
# Once published, users install with:
#   brew tap DENNIS-CODES/tap
#   brew install mpesa-dev

class MpesaDev < Formula
  desc "M-Pesa Daraja developer toolkit: doctor, inspect, tunnel, replay"
  homepage "https://github.com/DENNIS-CODES/mpesa-dev"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/DENNIS-CODES/mpesa-dev/releases/download/v#{version}/mpesa-dev-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_OF_AARCH64_APPLE_DARWIN_TARBALL"
    else
      url "https://github.com/DENNIS-CODES/mpesa-dev/releases/download/v#{version}/mpesa-dev-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_OF_X86_64_APPLE_DARWIN_TARBALL"
    end
  end

  on_linux do
    url "https://github.com/DENNIS-CODES/mpesa-dev/releases/download/v#{version}/mpesa-dev-x86_64-unknown-linux-musl.tar.gz"
    sha256 "REPLACE_WITH_SHA256_OF_X86_64_LINUX_MUSL_TARBALL"
  end

  def install
    bin.install "mpesa-dev"
    bin.install "mpesa-relay" if File.exist?("mpesa-relay")
  end

  test do
    system "#{bin}/mpesa-dev", "--version"
  end
end
