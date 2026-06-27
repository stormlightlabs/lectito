# lectito-mcp

`lectito-mcp` is a local stdio MCP server for article search and reading. MCP
clients start the binary as a subprocess and exchange newline-delimited
JSON-RPC messages over stdin and stdout.

Install it from crates.io:

```sh
cargo install lectito-mcp
```

The binary is named `lectito-mcp`.

## Tools

The server exposes two tools:

- `search_articles`: search DuckDuckGo HTML results and return titles, URLs,
  and snippets.
- `read_article`: fetch a public HTTP(S) URL and extract readable article
  content with Lectito.

`read_article` supports `markdown`, `text`, `html`, and `json` output chunks.
Use `offset` and `maxChars` to page through long results.

## Smoke Test

```sh
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' \
  | lectito-mcp
```

The server writes protocol messages to stdout. Keep logs on stderr if you wrap
or supervise the process.

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
