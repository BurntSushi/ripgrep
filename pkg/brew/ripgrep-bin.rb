class RipgrepBin < Formula
  version '16.0.0'
  desc "Recursively search directories for a regex pattern."
  homepage "https://github.com/BurntSushi/ripgrep"

  if OS.mac?
      url "https://github.com/BurntSushi/ripgrep/releases/download/#{version}/ripgrep-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "aa9183b0e0bfac03c5818895ad18b75001c402d230063898abfe5a58cd413caa"
  elsif OS.linux?
      url "https://github.com/BurntSushi/ripgrep/releases/download/#{version}/ripgrep-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "54268ab4643c00a290a29b0086066d64b3f56236dd805ec64629a86f9bd29fd0"
  end

  conflicts_with "ripgrep"

  def install
    bin.install "rg"
    man1.install "doc/rg.1"

    bash_completion.install "complete/rg.bash"
    zsh_completion.install "complete/_rg"
  end
end
