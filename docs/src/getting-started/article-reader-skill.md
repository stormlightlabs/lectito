# Article Reader Skill

Lectito includes an `article-reader` skill for agents that understand
`SKILL.md` directories (implement [agent skills](https://agentskills.io/home)).

The skill tells an agent when to search for articles, when to read a URL, how to
cite article-specific claims, and when to fall back to the Lectito CLI.

The source lives in the repository:

- [skills/article-reader][skill-dir]
- [SKILL.md][skill-md]
- [Lectito skill reference][skill-reference]
- [MCP server source][mcp-lib]
- [DuckDuckGo search source][ddg-source]

The skill is useful on its own, but it works best with the
[`lectito-mcp`](./mcp.md) server because the MCP server exposes the
`search_articles` and `read_article` tools named by the skill.

## Install From Source

Clone the repository and point your agent at the skill directory:

```sh
git clone https://github.com/stormlightlabs/lectito.git
cd lectito
```

Use this path in agents that accept a skill directory:

```text
/path/to/lectito/skills/article-reader
```

For Pi, load the skill explicitly with `--skill`:

```sh
pi --skill /path/to/lectito/skills/article-reader
```

For a one-shot Pi run:

```sh
pi --skill /path/to/lectito/skills/article-reader \
  "Find and summarize articles about Rust parser libraries"
```

You can also simply point pi to the skill's [source][skill-dir] and it'll handle
creating it for you.

If the MCP tools are available, the skill should use them. If they are not,
the skill reference tells the agent how to use the Lectito CLI for a known URL
and when to use another approved search mechanism.

## Pair With MCP

Install the MCP server:

```sh
cargo install lectito-mcp
```

Then configure your MCP-capable agent with:

```text
lectito-mcp
```

For local development, build the MCP server from the same checkout:

```sh
cargo build -p lectito-mcp --release
```

and point your agent at:

```text
/path/to/lectito/target/release/lectito-mcp
```

Keep the skill and the MCP server on the same revision when possible. The
skill names the tool contract, while the MCP crate implements it.

[skill-dir]: https://github.com/stormlightlabs/lectito/tree/main/skills/article-reader
[skill-md]: https://github.com/stormlightlabs/lectito/blob/main/skills/article-reader/SKILL.md
[skill-reference]: https://github.com/stormlightlabs/lectito/blob/main/skills/article-reader/references/lectito.md
[mcp-lib]: https://github.com/stormlightlabs/lectito/blob/main/crates/mcp/src/lib.rs
[ddg-source]: https://github.com/stormlightlabs/lectito/blob/main/crates/mcp/src/ddg.rs
