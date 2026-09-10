# Spire Tagger

Rust web app for browsing the Slay the Spire card/tag database.

## Run locally

```powershell
cargo run
```

Defaults:

- Bind address: `127.0.0.1:8000`
- Database path: `assets/spire_tagger.db`

Override with environment variables:

```powershell
$env:SPIRE_BIND = "127.0.0.1:8010"
$env:SPIRE_DB = "assets/spire_tagger.db"
cargo run
```

## Deployment Shape

Build a release binary and run it behind your public reverse proxy:

```bash
cargo build --release
SPIRE_BIND=127.0.0.1:8000 SPIRE_DB=/opt/spire-tagger/spire_tagger.db ./target/release/spire-tagger
```

For an LXC deployment, keep the app bound to a private interface or localhost, then have your Rust webserver terminate TLS and reverse proxy to `SPIRE_BIND`.

## Structure

- `src/domain.rs`: typed card model, enums, sorting behavior
- `src/data.rs`: SQLite catalog loading
- `src/tags.rs`: tag inheritance graph
- `src/query.rs`: Scryfall-like card query language
- `src/render.rs`: server-side HTML rendering
- `src/web.rs`: Axum routes
- `assets/`: runtime database and bundled CSS
