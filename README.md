# ai3d

`ai3d` is a small shared 3D scratchpad for an AI agent and a person using a Ghostty pane. It is a viewer bridge, not CAD: scenes are JSON, rasterminal handles interactive rendering, and the only human controls are orbit, zoom, and pan-like camera movement.

## Architecture

```text
scene.json
  -> Rust ai3d CLI / watcher
  -> trimesh primitive meshes + GLB export
  -> patched rasterminal (same-process GLB reload + camera JSON)
  -> Kitty Graphics Protocol
  -> Ghostty / Herdr right pane
```

The project does not implement a renderer or primitive triangulation. Python is only a thin adapter around `trimesh`; CLI, process ownership, hot reload, runtime state, and Herdr integration are Rust. rasterminal is pinned to commit `4dc56a378eb0094e2ad5991b3e0bce08580125db` and modified at build time by `scripts/rasterminal-reload.patch`.

## Setup

```sh
cd /Users/neam/ai3d
sh scripts/setup_rasterminal.sh
cargo install --path . --force
```

Setup creates a project-local `.venv`, installs `trimesh`, `numpy`, and `scipy`, clones the pinned rasterminal source, applies the small reload/state patch, and builds rasterminal with CMake.

## Usage

```sh
ai3d build examples/basic.json
ai3d open examples/basic.json
ai3d state
ai3d show examples/all-primitives.json
ai3d close
```

- `build` writes a GLB next to the source JSON.
- `open` stages the scene and, inside Herdr, creates a right split and runs the viewer there. Outside Herdr it prints the manual viewer command.
- `show` switches the existing viewer to another scene without replacing the rasterminal process.
- Saving the active JSON recompiles a stable runtime GLB. Patched rasterminal notices the atomic replacement, reloads it in the same process, and preserves the current camera.
- `state` merges canonical scene objects with live camera state (`azimuth`, `elevation`, `distance`, `target`, and quaternion).
- `close` closes only the pane recorded by this ai3d runtime.

Scene primitives are `sphere`, `cube`, `cylinder`, `cone`, `line`, and `arrow`. Mesh objects accept `position`, XYZ degree `rotation`, `scale`, `color`, and `opacity`. Lines and arrows accept `from`/`to`; their object transform is applied afterward. A top-level `arrows` array is also accepted for the compact form shown in `examples/basic.json`.

rasterminal controls are mouse drag or WASD/arrows to orbit, scroll or `+`/`-` to zoom, and `Q` to quit.

## Runtime and limitations

Runtime files live under the platform runtime directory (currently `/tmp/ai3d`): `runtime.json`, stable `scene.glb`, and `camera.json`.

`ai3d screenshot` intentionally reports an unsupported Phase 2 feature. Herdr can inspect PTY text and pane geometry but cannot return the actual pixels of one Ghostty pane; Kitty images are not present in the text buffer. A future pane-vision helper needs ScreenCaptureKit/CoreGraphics and reliable pane-to-screen coordinate mapping.

Only one ai3d viewer session is managed per user runtime directory. rasterminal's documented tmux/GNU screen limitation still applies; the tested Herdr/Ghostty direct PTY path supports Kitty graphics.
