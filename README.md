# ai3d

Minimal Rust JSON scene compiler and Ghostty scratchpad bridge. `build` emits a GLB path, while `show`/`open` watch the JSON and rewrite the asset on save. `open` uses Herdr when `HERDR_ENV` is present. Runtime metadata is stored in `$XDG_RUNTIME_DIR/ai3d-runtime.json` (or `/tmp`).

```sh
cargo run -- build examples/basic.json
cargo run -- open examples/basic.json
cargo run -- state
cargo run -- screenshot
```

The intended viewer is PavolUlicny/rasterminal (MIT, pinned externally by the environment); run it in the right pane against the generated `.glb`. The compiler keeps the scene JSON canonical and deliberately has no labels or editor UI. Pane pixel screenshots are unsupported because Kitty graphics do not appear in PTY text. This repository is an MVP scaffold; rasterminal integration and camera persistence require the upstream viewer patch described by the parent task.
