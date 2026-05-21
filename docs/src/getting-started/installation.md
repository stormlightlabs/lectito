# Installation

Lectito is split into a core library and a CLI. Use the library when your
application already has HTML. Use the CLI for local inspection, fixtures, shell
scripts, and quick conversions.

## Library

Add `lectito-core` to your Rust project:

```toml
[dependencies]
lectito-core = "0.1"
```

For local development against this workspace:

```toml
[dependencies]
lectito-core = { path = "crates/core" }
```

The core crate has no runtime service requirement. It parses the string you
pass in and returns an article result.

## CLI

Install the CLI from this workspace:

```sh
cargo install --path crates/cli
```

The binary is named `lectito`.

```sh
lectito --help
```

The CLI can read from a file, stdin, or a URL. URL support is a command-line
convenience; it is not part of the core library contract.

## License

Lectito is licensed under MPL-2.0.
