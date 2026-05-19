# OpenAuth Full App Example

This example is the living integration app for the current workspace version of
OpenAuth. It uses local path dependencies, so it tracks the repository API
instead of a published crate version.

## Run with SQLite

```bash
cargo run -p openauth-example-full-app
```

Open http://127.0.0.1:3000.

The default SQLite database is created at `examples/full-app/data/openauth.sqlite`.
The `data/` directory is local development state and should not be committed.

## Run with Docker services

From the repository root:

```bash
docker compose up -d postgres mysql redis valkey
```

Postgres:

```bash
OPENAUTH_EXAMPLE_DB=postgres \
DATABASE_URL=postgres://user:password@127.0.0.1:5432/openauth \
cargo run -p openauth-example-full-app
```

MySQL:

```bash
OPENAUTH_EXAMPLE_DB=mysql \
DATABASE_URL=mysql://user:password@127.0.0.1:3306/openauth \
cargo run -p openauth-example-full-app
```

Redis rate limiting:

```bash
OPENAUTH_EXAMPLE_RATE_LIMIT=redis \
REDIS_URL=redis://127.0.0.1:6379 \
cargo run -p openauth-example-full-app
```

Valkey rate limiting:

```bash
OPENAUTH_EXAMPLE_RATE_LIMIT=valkey \
VALKEY_URL=valkey://127.0.0.1:6380 \
cargo run -p openauth-example-full-app
```

Database-backed rate limiting:

```bash
OPENAUTH_EXAMPLE_DB=sqlite \
OPENAUTH_EXAMPLE_RATE_LIMIT=database \
cargo run -p openauth-example-full-app
```

## Configuration

| Variable | Default |
| --- | --- |
| `OPENAUTH_EXAMPLE_HOST` | `127.0.0.1` |
| `OPENAUTH_EXAMPLE_PORT` | `3000` |
| `OPENAUTH_EXAMPLE_BASE_URL` | `http://127.0.0.1:3000/api/axum/auth` |
| `OPENAUTH_SECRET` | development-only secret |
| `OPENAUTH_EXAMPLE_DB` | `sqlite` |
| `DATABASE_URL` | backend-specific local URL |
| `OPENAUTH_EXAMPLE_RATE_LIMIT` | `memory` |
| `REDIS_URL` | `redis://127.0.0.1:6379` |
| `VALKEY_URL` | `valkey://127.0.0.1:6380` |

Supported `OPENAUTH_EXAMPLE_DB` values are `memory`, `sqlite`, `postgres`, and
`mysql`. Supported `OPENAUTH_EXAMPLE_RATE_LIMIT` values are `memory`,
`database`, `redis`, and `valkey`.

MongoDB and MSSQL are intentionally not wired into this example yet because the
workspace does not currently expose OpenAuth adapters for them.
