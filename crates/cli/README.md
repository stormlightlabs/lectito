# lectito

A CLI tool for extracting readable content from web pages.

## Installation

### Install via cargo

```sh
cargo install lectito-cli
```

The binary will be installed as `lectito`.

### Install from source

```sh
git clone https://github.com/stormlightlabs/lectito.git
cd lectito
cargo install --path crates/cli
```

## Usage

Extract content from a URL:

```sh
lectito https://example.com/article
```

Extract from a local HTML file:

```sh
lectito article.html
```

Extract from stdin:

```sh
cat page.html | lectito -
```

For more options and features, see the [main project README](https://github.com/stormlightlabs/lectito) or run:

```sh
lectito --help
```
