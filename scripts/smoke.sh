#!/usr/bin/env bash
set -euo pipefail

skip_live=false
if [[ "${1:-}" == "--skip-live" ]]; then
  skip_live=true
fi

out_dir="${LECTITO_SMOKE_OUT_DIR:-target/pre-release-smoke}"
mkdir -p "$out_dir"

run_case() {
  local name="$1"
  local input="$2"
  local format="$3"
  local output="$out_dir/$name.$format"

  echo "Writing $output"
  cargo run -p lectito-cli -- "$input" "--$format" > "$output"
}

run_case "article-quanta" \
  "crates/fixtures/samples/test-pages/quanta-1/source.html" \
  "text"

run_case "reference-wikipedia" \
  "crates/fixtures/samples/test-pages/wikipedia/source.html" \
  "text"

run_case "news-nytimes" \
  "crates/fixtures/samples/test-pages/nytimes-1/source.html" \
  "markdown"

run_case "code-mintlify" \
  "crates/fixtures/samples/test-pages/codeblocks--mintlify/source.html" \
  "markdown"

if [[ "$skip_live" == false ]]; then
  run_case "live-rust-blog" \
    "https://blog.rust-lang.org/2024/11/28/Rust-1.83.0.html" \
    "text"
fi

cat <<MSG

Review the files in $out_dir before release. Check for:

- missing article text
- navigation, table-of-contents, or "continue reading" chrome
- broken inline links or punctuation around links
- malformed code blocks, tables, media, footnotes, or math

Use --skip-live when network access is unavailable.
MSG
