#!/usr/bin/env python3
"""Keep the source-install tap using the same app bundler as local development."""
import base64
import hashlib
import json
import re
import subprocess
import sys
import urllib.request


def main(tag):
    if not re.fullmatch(r"v\d+\.\d+\.\d+", tag):
        raise ValueError("Expected stable version tag")
    url = f"https://github.com/cameroncooper/zbase/archive/refs/tags/{tag}.tar.gz"
    with urllib.request.urlopen(url) as response:
        digest = hashlib.file_digest(response, "sha256").hexdigest()
    formula = '''class Zbase < Formula
  desc "A fast, native chat client"
  homepage "https://github.com/cameroncooper/zbase"
  url "URL"
  sha256 "DIGEST"
  license "MIT"

  depends_on "rust" => :build
  depends_on "cmake" => :build
  depends_on "pkgconf" => :build
  depends_on macos: :sequoia
  depends_on "ffmpeg"

  def install
    system "scripts/macos/build_app_bundle.sh", "release"
    prefix.install "dist/macos/zBase.app"
    bin.install_symlink prefix/"zBase.app/Contents/MacOS/zbase"
  end

  def caveats
    <<~EOS
      The zBase.app bundle is at #{prefix}/zBase.app.
      Official signed and notarized DMGs are available on GitHub Releases.
    EOS
  end

  test do
    assert_predicate bin/"zbase", :executable?
  end
end
'''.replace("URL", url).replace("DIGEST", digest)
    endpoint = "repos/cameroncooper/homebrew-tap/contents/Formula/zbase.rb"
    # Fail on lookup/authentication errors rather than assuming the existing formula is absent.
    existing = json.loads(subprocess.check_output(["gh", "api", endpoint], text=True))
    payload = {"message": f"Update zbase formula to {tag}", "content": base64.b64encode(formula.encode()).decode(), "sha": existing["sha"]}
    subprocess.run(["gh", "api", "--method", "PUT", endpoint, "--input", "-", "--silent"], input=json.dumps(payload), text=True, check=True)


if __name__ == "__main__":
    main(sys.argv[1])
