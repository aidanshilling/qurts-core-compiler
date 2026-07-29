# playground

Local web app that visualizes qurts source across each implemented lowering stage
(source → CST → pass-1 MLIR, with more stages appended automatically as `lower` grows).

## Build & run

```bash
cd playground/frontend
npm install
npm run build

cd ..
cargo run -p playground
```

Open the printed `http://127.0.0.1:3000`.

## Development

Run the frontend dev server (hot reload) and the API server side by side; Vite proxies
`/api/*` to the Rust server:

```bash
cd playground/frontend && npm run dev   # in one terminal
cargo run -p playground                 # in another
```

## API

`POST /api/compile` `{"source": "..."}` → `{"ok": true, "stages": [...]}` or
`{"ok": false, "error": "..."}`. Each stage is `{"id", "label", "kind": "text", "content"}` or
`{"id", "label", "kind": "functions", "functions": [{"name", "ok", "content"}]}` — one entry per
function, since pass 1 lowers a program's functions independently and some may succeed while
others are rejected. See `src/compile.rs`.

`GET /api/examples` → the bundled `parser/examples/scripts/*.qurts` files, for the dropdown.
