#!/usr/bin/env bash
# Run the lectito API hurl test files.
#
# Requires `hurl` (https://hurl.dev) and a running API server
# at http://localhost:3000 (start it with `just api-run`).
#
# Usage:
#   scripts/api.sh             # local-only files (no network)
#   scripts/api.sh --all       # include network-dependent happy paths
#   scripts/api.sh --verbose   # forward --verbose to hurl
#   scripts/api.sh healthz     # run a single file by stem or path
#   scripts/api.sh --help
#
# The happy paths in extract.hurl and evaluate.hurl fetch
# https://en.wikipedia.org, so they need network access.
#
# Every error case is local and runs by default.
#
# Pass --all (or any network-capable flag) to include the live requests.
set -euo pipefail

api_dir="scripts/api"
base_url="${LECTITO_API_URL:-http://localhost:3000}"
hurl_args=()
files=()
include_live=false

print_help() {
    cat <<'USAGE'
scripts/api.sh — run the lectito API hurl tests

Requires `hurl` (https://hurl.dev) and a running API server.
Start the server in another terminal with:  just api-run

Usage:
  scripts/api.sh [options] [file ...]

Options:
  --all           Include network-dependent happy paths (extract/evaluate).
  --verbose       Forward --verbose to hurl (full request/response logs).
  --very-verbose   Forward --very-verbose to hurl.
  --color         Force colorized hurl output.
  --help, -h      Show this help and exit.

Arguments:
  file            A hurl file stem (e.g. "healthz"), a bare name
                  (e.g. "healthz.hurl"), or a path. With no files given,
                  all local-only hurl files run.

Environment:
  LECTITO_API_URL  Base URL of the API server (default http://localhost:3000)

Notes:
  - Tests run sequentially (--jobs 1) so failures point at one file.
  - Every error response is asserted to use the structured shape:
    a JSON body {"error":{"code","message"}} plus an x-error-code header.
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
    --help | -h)
        print_help
        exit 0
        ;;
    --all)
        include_live=true
        shift
        ;;
    --verbose | --very-verbose | --color)
        hurl_args+=("$1")
        shift
        ;;
    --*)
        hurl_args+=("$1")
        shift
        ;;
    *)
        files+=("$1")
        shift
        ;;
    esac
done

if ! command -v hurl >/dev/null 2>&1; then
    echo "error: hurl is not installed. See https://hurl.dev/docs/installation.html" >&2
    exit 2
fi

# Resolve files into absolute paths against the api directory.
resolve_file() {
    local name="$1"
    if [[ -f "$name" ]]; then
        printf '%s\n' "$name"
        return
    fi
    local stripped="${name%.hurl}"
    if [[ -f "$api_dir/$stripped.hurl" ]]; then
        printf '%s\n' "$api_dir/$stripped.hurl"
        return
    fi
    echo "error: no hurl file matching '$name' in $api_dir" >&2
    exit 2
}

if [[ ${#files[@]} -gt 0 ]]; then
    resolved=()
    for f in "${files[@]}"; do
        resolved+=("$(resolve_file "$f")")
    done
    files=("${resolved[@]}")
else
    # Default: every local-only file. extract.hurl and evaluate.hurl each
    # contain a live happy path that fetches Wikipedia;
    #
    # we skip them unless --all (or an explicit file) asks for them.
    if $include_live; then
        files=("$api_dir"/*.hurl)
    else
        files=(
            "$api_dir/healthz.hurl"
            "$api_dir/openapi.hurl"
            "$api_dir/transform.hurl"
        )
    fi
fi

# Waits for the server to be reachable.
wait_for_server() {
    local tries=30
    while [[ $tries -gt 0 ]]; do
        if curl -sf -o /dev/null "$base_url/healthz" 2>/dev/null; then
            return 0
        fi
        tries=$((tries - 1))
        sleep 0.5
    done
    echo "error: API server not reachable at $base_url" >&2
    echo "       start it with: just api-run" >&2
    exit 1
}

if ! wait_for_server; then
    exit 1
fi

# When the default base url is used, the .hurl files already target
# localhost:3000, so no --variable injection is needed. If a custom
# LECTITO_API_URL is set, rewrite each file's localhost:3000 on the fly
# by piping through sed to retarget the base URL, then read from stdin.
run_hurl() {
    if [[ "$base_url" == "http://localhost:3000" ]]; then
        hurl --test --jobs 1 "${hurl_args[@]+"${hurl_args[@]}"}" "${files[@]}"
    else
        cat "${files[@]}" |
            sed "s#http://localhost:3000#$base_url#g" |
            hurl --test --jobs 1 "${hurl_args[@]+"${hurl_args[@]}"}" -
    fi
}

run_hurl
