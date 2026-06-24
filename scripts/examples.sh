#!/usr/bin/env bash
set -uo pipefail

examples_file="${LECTITO_EXAMPLES_FILE:-scripts/doc/examples.txt}"
out_dir="${LECTITO_EXAMPLES_OUT_DIR:-target/examples}"
bin="${LECTITO_BIN:-target/debug/lectito}"

if [[ ! -f "$examples_file" ]]; then
  echo "examples file not found: $examples_file" >&2
  exit 2
fi

if [[ ! -x "$bin" ]]; then
  cargo build -p lectito-cli || exit 2
fi

mkdir -p "$out_dir"
summary="$out_dir/summary.tsv"
: > "$summary"

url_count=0
failures=0

while IFS= read -r url; do
  url_count=$((url_count + 1))
  name="$(printf "%03d" "$url_count")"
  json="$out_dir/$name.inspect.json"
  err="$out_dir/$name.stderr.txt"

  printf "Inspecting %s %s\n" "$name" "$url"

  if "$bin" inspect "$url" --json --timeout 30 > "$json" 2> "$err"; then
    length="$(jq -r '.article.length // 0' "$json" 2> /dev/null || printf "0")"
    title="$(jq -r '.article.title // ""' "$json" 2> /dev/null || printf "")"
    printf "ok\t%s\t%s\t%s\n" "$name" "$length" "$url" >> "$summary"
    printf "  ok %s chars %s\n" "$length" "$title"
  else
    status=$?
    failures=$((failures + 1))
    printf "fail\t%s\t%s\t%s\n" "$name" "$status" "$url" >> "$summary"
    printf "  failed with exit %s\n" "$status"
  fi
done < <(awk '/^https?:\/\// { print }' "$examples_file")

printf "\nWrote %s\n" "$summary"

if [[ "$url_count" -eq 0 ]]; then
  echo "no URLs found in $examples_file" >&2
  exit 2
fi

if [[ "$failures" -gt 0 ]]; then
  printf "%s of %s examples failed\n" "$failures" "$url_count" >&2
  exit 1
fi

printf "All %s examples passed\n" "$url_count"
