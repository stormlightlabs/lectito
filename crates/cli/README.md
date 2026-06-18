# lectito-cli

`lectito-cli` is the command-line package for Lectito. It installs a `lectito`
binary backed by the `lectito` Rust library.

Install it from crates.io:

```sh
cargo install lectito-cli
```

Example commands:

```sh
lectito article.html
lectito https://example.com/article --json --pretty
lectito article.html --html
lectito readable article.html
lectito inspect article.html
lectito article.html --timeout 10
```

The CLI can read from files, stdin, and URLs. URL fetching is a CLI
convenience; the `lectito` library accepts HTML supplied by the caller.
