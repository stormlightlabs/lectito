# Installation

Lectito is split into a core library and a CLI. Use the library when your
application already has HTML. Use the CLI for local inspection, shell scripts,
and quick conversions.

## Library

Add `lectito` to your Rust project:

```toml
[dependencies]
lectito = "0.1"
```

For local development against this workspace:

```toml
[dependencies]
lectito = { path = "crates/core" }
```

The Rust crate name is `lectito`.

The core crate has no runtime service requirement. It parses the string you
pass in and returns an article result.

## CLI

Install the CLI from crates.io:

```sh
cargo install lectito-cli
```

For local development against this workspace:

```sh
cargo install --path crates/cli
```

The binary is named `lectito`.

```sh
lectito --help
```

The CLI can read from a file, stdin, or a URL. URL support is a command-line
convenience; it is not part of the core library contract.

Fixture helpers are workspace-only and are not part of the published CLI
package.

For local fixture inspection, run the unpublished workspace helper:

```sh
cargo run -p lectito-fixtures --bin lectito-fixture -- sample-name
```

## License

Lectito is licensed under MPL-2.0.
