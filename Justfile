set dotenv-load

default:
    just --list

fixtures *args:
    cargo run -p lectito-fixtures --bin corpus -- {{ args }}

fixtures-all *args:
    cargo run -p lectito-fixtures --bin corpus -- --all {{ args }}

fixture name *args:
    cargo run -p lectito-fixtures --bin corpus -- {{ name }} {{ args }}

script name *args:
    bash scripts/{{ name }}.sh {{ args }}

smoke *args:
    bash scripts/smoke.sh {{ args }}

smoke-skip-live:
    bash scripts/smoke.sh --skip-live

examples:
    bash scripts/examples.sh

# web/
web-install:
    pnpm --dir web install

web-dev:
    pnpm --dir web run dev

web-build:
    pnpm --dir web run build

web-build-wasm:
    pnpm --dir web run build:wasm

web-lint:
    pnpm --dir web run lint

web-format:
    pnpm --dir web run format

web-messages-extract:
    pnpm --dir web run messages:extract

web-messages-compile:
    pnpm --dir web run messages:compile

web-messages: web-messages-extract web-messages-compile

web-preview:
    pnpm --dir web run preview

api-build:
    cargo build -p lectito-api

api-test:
    cargo test -p lectito-api

api-run:
    cargo run -p lectito-api

api-fmt-check:
    cargo fmt --check -p lectito-api

api-docker-build:
    docker build -f crates/api/Dockerfile -t lectito-api .
