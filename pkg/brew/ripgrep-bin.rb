class RipgrepBin < Formula
  version '13.0.0'
  desc "Recursively search directories for a regex pattern."
  homepage "https://github.com/BurntSushi/ripgrep"

  if OS.mac?
      url "https://github.com/BurntSushi/ripgrep/releases/download/#{version}/ripgrep-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "585c18350cb8d4392461edd6c921e6edd5a97cbfc03b567d7bd440423e118082"
  elsif OS.linux?
      url "https://github.com/BurntSushi/ripgrep/releases/download/#{version}/ripgrep-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "ee4e0751ab108b6da4f47c52da187d5177dc371f0f512a7caaec5434e711c091"
  end

  conflicts_with "ripgrep"

  def install
    bin.install "rg"
    man1.install "doc/rg.1"

    bash_completion.install "complete/rg.bash"
    zsh_completion.install "complete/_rg"
  end
end
