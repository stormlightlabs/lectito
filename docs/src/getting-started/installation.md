# Installation

Lectito provides both a CLI tool and a Rust library. Install whichever fits your needs.

## CLI Installation

### From crates.io

The easiest way to install the CLI is via cargo:

```bash
cargo install lectito-cli
```

This installs the `lectito` binary in your cargo bin directory (typically `~/.cargo/bin`).

### From Source

```bash
# Clone the repository
git clone https://github.com/stormlightlabs/lectito.git
cd lectito

# Build and install
cargo install --path crates/cli
```

### Pre-built Binaries

Pre-built binaries are available on the [GitHub Releases page](https://github.com/stormlightlabs/lectito/releases) for Linux, macOS, and Windows.

Download the appropriate binary for your platform and place it in your PATH.

### Verify Installation

```bash
lectito --version
```

You should see version information printed.

## Library Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
lectito-core = "0.1"
```

Then run `cargo build` to fetch and compile the dependency.

### Feature Flags

The library has several optional features:

```toml
[dependencies]
lectito-core = { version = "0.1", features = ["fetch", "markdown"] }
```

| Feature      | Default | Description                       |
| ------------ | ------- | --------------------------------- |
| `fetch`      | Yes     | Enable URL fetching with reqwest  |
| `markdown`   | Yes     | Enable Markdown output format     |
| `siteconfig` | Yes     | Enable site configuration support |

If you don't need URL fetching (e.g., you have your own HTTP client), disable the default features:

```toml
[dependencies]
lectito-core = { version = "0.1", default-features = false, features = ["markdown"] }
```

## Development Build

To build from source for development:

```bash
# Clone the repository
git clone https://github.com/stormlightlabs/lectito.git
cd lectito

# Build the workspace
cargo build --release

# The CLI binary will be at target/release/lectito
```

## Next Steps

- [Quick Start Guide](quick-start.md) - Get started with basic usage
- [CLI Usage](cli-usage.md) - Learn CLI commands and options
- [Library Guide](../library/basic-usage.md) - Use Lectito as a library
