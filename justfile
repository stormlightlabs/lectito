format:
    cargo fmt

alias fmt := format

# Lint AND fix
lint:
    cargo clippy --fix --allow-dirty

compile:
    cargo check

# Overall code quality check
check: format lint compile test

# Finds comments
find-comments:
    rg -n --pcre2 '^\s*//(?![!/])' -g '*.rs'

alias cmt := find-comments

test:
    cargo test --quiet

# Run all tests including integration tests
test-all:
    cargo test --all-features --quiet

# Run integration tests only
test-integration:
    cargo test --all-features --quiet -- --test-threads=1 integration

# Run benchmarks
bench:
    cargo bench

# Run benchmarks with baseline comparison
bench-compare baseline="main":
    cargo bench -- --baseline {{baseline}}

# Generate shell completion scripts
completions:
    #!/usr/bin/env bash
    set -e
    mkdir -p completions
    cargo run --release -- --completions bash > completions/lectito.bash
    cargo run --release -- --completions zsh > completions/_lectito
    cargo run --release -- --completions fish > completions/lectito.fish
    cargo run --release -- --completions powershell > completions/lectito.ps1
    echo "Completions generated in completions/"

# Build mdbook documentation
docs-build:
    mdbook build docs/

# Serve documentation locally (opens browser)
docs-serve:
    mdbook serve docs/ --open

# Serve documentation without opening browser
docs:
    mdbook serve docs/

# Test mdbook code examples
docs-test:
    mdbook test docs/

alias doc := docs
