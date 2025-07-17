# Bearmark

[![codecov](https://codecov.io/github/linw1995/bearmark/graph/badge.svg?token=F2G2WCN6OP)](https://codecov.io/github/linw1995/bearmark)

Bearmark is a lightweight browser bookmark management system,
designed for developers who need
a personalized solution for managing bookmarks through API integration.

It allows users to deploy and use it on their servers,
ensuring the security and privacy of data.

## Deployments

- [docker-compose](./deployments/docker-compose/)

### Securities

Supports simple API key authentication.
You can use below command to generate a random API key.

```bash
openssl rand -base64 32
```

> [!CAUTION]
> If you use bearmark in the public network, please use HTTPS.

### Use docker-compose for deployment

```bash
cd ./deployments/docker-compose/
cp .env.example .env # and modify the .env file

docker compose up -d

open http://localhost:2284
```

#### Environment Variables

- `BM_API_KEY`: API key for authentication
- `BM_UI_PATH`: Path to web UI static files (default: `./static` in container)
- `BM_DATABASES_MAIN_URL`: Database connection URL

The web interface is automatically served at the root path when `BM_UI_PATH` is configured.

## Integrations

- **Web UI**: Built-in web interface (`bearmark-web`)

  The web interface is integrated within the Bearmark server.
  Access it at <http://localhost:2284/> (or root path of your server).
  Input your API key to start managing bookmarks.

- Raycast Extension: [bearmark-raycast](https://github.com/linw1995/bearmark_raycast)

  Enable searching and opening bookmarks in Raycast.
  Save the currently viewed Safari webpage to Bearmark.
  Configure your Bearmark server API endpoint <http://localhost:2284/api>
  and API key in the Raycast extension.

- Auto-generated OpenAPI documentation

  Already integrated with Bearmark server.
  Open the link <http://localhost:2284/swagger-ui/> in your browser.

## Developments

Use nix to setup development environment,
or use `direnv` to load nix environment automatically.

```bash
# setup database by docker compose
# and generate .envrc file
./scripts/cli.sh setup

# activate manually
nix develop
source .envrc
```

### Web UI Development

The web interface is built with Leptos and served by the API server.

```bash
# Build the web interface
trunk build

# Set environment variable and run server with integrated web UI
export BM_UI_PATH=./dist
./scripts/cli.sh serve

# For development with hot-reload (separate terminals)
trunk serve  # Web UI development server on port 1420
./scripts/cli.sh serve  # API server on port 8000
```

### Helpful Scripts

Also, you can use `./scripts/cli.sh` to invoke startups and testings.

```bash
./scripts/cli.sh install-deps

./scripts/cli.sh setup # setup database by docker compose
./scripts/cli.sh teardown # cleanup

./scripts/cli.sh lint
./scripts/cli.sh test
./scripts/cli.sh coverage
```

### Cross Compiling

You can use the following command to cross compile the binary.

Or use pre-built docker image [ghcr.io/linw1995/bearmark](https://github.com/linw1995/bearmark/pkgs/container/bearmark)

```bash
nix develop .#x86_64-unknown-linux-musl

cargo build --target x86_64-unknown-linux-musl --release \
  --package bearmark-api --bin serve
echo `find target -name serve -type f`
```
