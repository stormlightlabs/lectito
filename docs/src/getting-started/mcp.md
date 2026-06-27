# MCP Server

`lectito-mcp` is a local stdio MCP server for article search and reading.
MCP clients start it as a subprocess and exchange JSON-RPC messages over
stdin and stdout.

The server exposes two tools:

| Tool              | Description                                                                   |
| ----------------- | ----------------------------------------------------------------------------- |
| `search_articles` | search DuckDuckGo HTML results and return titles, URLs, and snippets.         |
| `read_article`    | fetch a public HTTP(S) URL and extract readable article content with Lectito. |

Because this is a stdio server, keep stdout reserved for MCP messages.

Write logs to stderr if you wrap or supervise the binary.

For agent workflow instructions, see the
[Article Reader Skill](./article-reader-skill.md). The skill points agents at
`search_articles` and `read_article` when those MCP tools are available.

## Build

Install the published MCP server with Cargo:

```sh
cargo install lectito-mcp
```

The binary will be available as:

```text
lectito-mcp
```

Build the release binary from the repository root:

```sh
cargo build -p lectito-mcp --release
```

The binary will be at:

```text
target/release/lectito-mcp
```

For local development, you can build the debug binary:

```sh
cargo build -p lectito-mcp
```

Then run a minimal tools-list smoke test:

```sh
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' \
  | target/debug/lectito-mcp
```

## Configuration

Set these environment variables from your MCP client config when you need to
change the defaults:

| Variable                             |   Default | Purpose                                    |
| ------------------------------------ | --------: | ------------------------------------------ |
| `LECTITO_MCP_DEFAULT_SEARCH_RESULTS` |       `5` | Default `search_articles` result count.    |
| `LECTITO_MCP_MAX_SEARCH_RESULTS`     |      `10` | Upper bound for `search_articles.limit`.   |
| `LECTITO_MCP_MAX_FETCH_BYTES`        | `2097152` | Maximum bytes read while fetching a page.  |
| `LECTITO_MCP_REDIRECT_LIMIT`         |       `5` | Maximum redirect hops for article fetches. |
| `LECTITO_MCP_REQUEST_TIMEOUT_SECS`   |      `20` | HTTP request timeout in seconds.           |
| `LECTITO_MCP_MAX_ARTICLE_CHARS`      |   `30000` | Upper bound for `read_article.maxChars`.   |
| `LECTITO_MCP_ALLOW_PRIVATE_NETWORK`  |   `false` | Allow private or loopback fetch targets.   |

Leave `LECTITO_MCP_ALLOW_PRIVATE_NETWORK=false` unless you intentionally want
the server to read URLs from local or private networks.

## Codex

Codex supports stdio MCP servers in the CLI and IDE extension. The durable
configuration lives in `~/.codex/config.toml`, or in a trusted repo-local
`.codex/config.toml`.

The simplest install is:

```sh
codex mcp add lectito -- lectito-mcp
```

If you are using a local checkout instead of the Cargo-installed binary, point
Codex at the built executable:

```sh
codex mcp add lectito -- /path/to/lectito/target/release/lectito-mcp
```

For a checked-in project config with the Cargo-installed binary, add a table
like this:

```toml
[mcp_servers.lectito]
command = "lectito-mcp"
startup_timeout_sec = 10
tool_timeout_sec = 60

[mcp_servers.lectito.env]
LECTITO_MCP_MAX_SEARCH_RESULTS = "10"
LECTITO_MCP_DEFAULT_SEARCH_RESULTS = "5"
```

During development, run through Cargo only after the crate has been built at
least once:

```toml
[mcp_servers.lectito]
command = "cargo"
args = ["run", "-p", "lectito-mcp", "--quiet"]
cwd = "/path/to/lectito"
```

Restart Codex after editing `config.toml`. In the Codex TUI, use `/mcp` to
check whether the server connected.

## Claude Code

Claude Code can add local stdio MCP servers with `claude mcp add`. Put `--`
before the server command so Claude does not parse server flags as Claude
flags.

```sh
claude mcp add --transport stdio lectito \
  -- lectito-mcp
```

To share the setup with a project, create `.mcp.json` in the repo:

```json
{
  "mcpServers": {
    "lectito": {
      "type": "stdio",
      "command": "lectito-mcp",
      "env": {
        "LECTITO_MCP_MAX_SEARCH_RESULTS": "10",
        "LECTITO_MCP_DEFAULT_SEARCH_RESULTS": "5"
      }
    }
  }
}
```

If you prefer to run through Cargo while developing:

```sh
claude mcp add --transport stdio lectito \
  -- cargo run -p lectito-mcp --quiet
```

Run that command from the repository root, or use a project `.mcp.json` with an
absolute binary path.

## Pi Coding Agent

Pi does not include built-in MCP support. Its documented extension model is
skills, prompt templates, TypeScript extensions, and packages. For Pi, install
the article-reading workflow as a skill, then use the Lectito CLI or a custom
Pi extension for the backend.

Pi can load skill directories with `--skill`. Use the checked-in
`article-reader` skill to teach Pi when to search for articles, read URLs, and
cite sources:

```sh
pi --skill /path/to/lectito/skills/article-reader
```

For a one-shot run:

```sh
pi --skill /path/to/lectito/skills/article-reader \
  "Read and summarize https://example.com/article"
```

For persistent discovery, place the skill under one of Pi's skill locations,
such as `.pi/skills/`, `.agents/skills/`, `~/.pi/agent/skills/`, or
`~/.agents/skills/`.

If you are building a Pi extension that bridges to MCP, build `lectito-mcp`
from the same checkout and launch this stdio command from the extension:

```sh
cargo build -p lectito-mcp --release
```

```text
/path/to/lectito/target/release/lectito-mcp
```

The local Pi CLI used while writing this page exposes `pi install <source>`
for packages and `--skill <path>` for skill directories, but
[no MCP subcommand](https://pi.dev/docs/latest/usage#design-principles).

## Sources

- [Codex MCP documentation](https://developers.openai.com/codex/mcp)
- [Claude Code MCP documentation](https://docs.anthropic.com/en/docs/claude-code/mcp)
- [Model Context Protocol](https://modelcontextprotocol.io/)
- Pi CLI help and installed docs: `pi --help`, `pi install --help`,
  `docs/skills.md`, and `docs/usage.md` from `@earendil-works/pi-coding-agent`
