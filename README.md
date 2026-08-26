# ai3d

`ai3d` is a small shared 3D scratchpad for an AI agent and a person using a Ghostty pane. It is a viewer bridge, not CAD: scenes are JSON, rasterminal handles interactive rendering, and the human controls are orbit and zoom.

## Architecture

```text
scene.json
  -> Rust ai3d CLI / watcher
  -> trimesh primitive/custom meshes + GLB export
  -> patched rasterminal (same-process GLB reload + camera JSON)
  -> half-block terminal graphics (`--graphics blocks`)
  -> Ghostty / Herdr right pane
```

The project does not implement a renderer or primitive triangulation. Python is a thin adapter around `trimesh`; CLI, process ownership, hot reload, runtime state, and Herdr integration are Rust. rasterminal is pinned to commit `4dc56a378eb0094e2ad5991b3e0bce08580125db` and modified at build time by `scripts/rasterminal-reload.patch`.

Herdr proxies the PTY. In the verified Ghostty/Herdr path, Kitty graphics capability negotiation/APC image data does not survive that proxy: rasterminal can keep drawing its text HUD while the image viewport remains black. `ai3d` therefore launches rasterminal with `--graphics blocks` by default. Set `AI3D_GRAPHICS` only when deliberately testing another rasterminal backend.

## Setup

The currently verified checkout is `/Volumes/NEAM_SSD/ai3d`:

```sh
cd /Volumes/NEAM_SSD/ai3d
sh scripts/setup_rasterminal.sh
cargo install --path . --force
```

Setup creates a project-local `.venv`, installs `trimesh`, `numpy`, and `scipy`, clones the pinned rasterminal source, applies the reload/state patch, and builds rasterminal with CMake.

The current binary still uses its compile-time Cargo manifest directory to locate `.venv`, `scripts/scene_to_glb.py`, and the default vendored rasterminal binary. If the checkout is moved, rebuild/reinstall `ai3d` from the new location. `AI3D_RASTERMINAL` can override only the rasterminal executable path.

## Usage

```sh
ai3d build examples/basic.json
ai3d open examples/basic.json
ai3d state
ai3d show examples/all-primitives.json
ai3d screenshot
ai3d close
```

- `build` writes a GLB next to the source JSON.
- `open` stages the scene and, inside Herdr, creates a right split and runs the viewer there. Outside Herdr it prints the manual viewer command.
- `show` switches the existing viewer to another scene without replacing the rasterminal process.
- Saving the active JSON recompiles a stable runtime GLB. Patched rasterminal notices the atomic replacement, reloads it in the same process, and preserves the current camera.
- `state` merges canonical scene objects with live camera state (`azimuth`, `elevation`, `distance`, `target`, and quaternion).
- `screenshot` requests the active renderer framebuffer to write `/tmp/ai3d/current.png` and returns its path, camera, and scene metadata.
- `close` stops the managed pane/viewer and removes only ai3d runtime artifacts. It is safe to run repeatedly; a later `open` cleans stale ai3d state when the recorded viewer is gone.

Scene primitives are `sphere`, `cube`, `cylinder`, `cone`, `line`, `arrow`, and custom `mesh`. Mesh objects accept `vertices`/`faces` plus `position`, XYZ-degree `rotation`, `scale`, `color`, and `opacity`. Cylinders can use normal `position`/`scale` or `from`/`to` endpoints. Lines and arrows accept `from`/`to`; a top-level `arrows` array is also accepted for the compact form shown in `examples/basic.json`.

rasterminal controls are mouse drag or WASD/arrows to orbit, scroll or `+`/`-` to zoom, and `Q` to quit.

## Runtime and limitations

Runtime files live under `/tmp/ai3d`: `runtime.json`, stable `scene.glb`, `camera.json`, screenshot request state, and the current framebuffer PNG when requested.

`ai3d screenshot` is useful for renderer/debug metadata, but an internal framebuffer PNG is **not** proof that the terminal actually displayed the geometry. The release acceptance test for terminal visibility is an OS-level capture of the Ghostty window/pane (for example, macOS `screencapture`) showing the expected geometry. This distinction caught the Kitty/Herdr failure where the framebuffer contained geometry while the real pane stayed black.

Only one ai3d viewer session is managed per user runtime directory. `ai3d show` preserves the current camera; when switching between scenes with very different scales, use `ai3d close` followed by `ai3d open` to get automatic framing again.

## Verification

The verified Herdr/Ghostty release gate has rendered a cube, three cylinders, and a DNA double helix in the actual right pane using the half-block backend. The repository CI performs portable source-level checks (`cargo fmt`, `clippy`, tests/build, and Python syntax compilation), but it cannot replace the macOS Ghostty/Herdr OS-level rendering acceptance test.
