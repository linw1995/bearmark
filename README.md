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

## Integrations

- Web viewer: [bearmark-web](https://github.com/linw1995/bearmark_web)

  Already integrated with Bearmark server.
  Open the link <http://localhost:2284/> in your browser,
  and input your API key.

- Raycast Extension: [bearmark-raycast](https://github.com/linw1995/bearmark_raycast)

  Enable searching and opening bookmarks in Raycast.
  Save the currently viewed Safari webpage to Bearmark.
  Configure your Bearmark server API endpoint <http://localhost:2284/api>
  and API key in the Raycast extension.
