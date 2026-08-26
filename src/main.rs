use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, SystemTime},
};

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Build {
        scene: PathBuf,
    },
    Show {
        scene: PathBuf,
    },
    Open {
        scene: PathBuf,
    },
    State,
    Close,
    Screenshot,
    #[command(name = "__viewer", hide = true)]
    Viewer,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scene {
    #[serde(default)]
    pub objects: Vec<Object>,
    #[serde(default)]
    pub arrows: Vec<Arrow>,
    #[serde(default)]
    pub camera: Option<Camera>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Object {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation: [f32; 3],
    #[serde(default = "one")]
    pub scale: [f32; 3],
    #[serde(default)]
    pub color: String,
    #[serde(default = "opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub from: Option<[f32; 3]>,
    #[serde(default)]
    pub to: Option<[f32; 3]>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Arrow {
    pub from: [f32; 3],
    pub to: [f32; 3],
    #[serde(default)]
    pub color: String,
    #[serde(default = "opacity")]
    pub opacity: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Camera {
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
    pub target: [f32; 3],
    #[serde(default)]
    pub quaternion: Option<[f32; 4]>,
}

fn one() -> [f32; 3] {
    [1.; 3]
}
fn opacity() -> f32 {
    1.
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Runtime {
    scene: Option<String>,
    glb: Option<String>,
    camera_state: Option<String>,
    pane: Option<String>,
    viewer_pid: Option<u32>,
}

fn runtime_dir() -> PathBuf {
    dirs::runtime_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("ai3d")
}
fn runtime_path() -> PathBuf {
    runtime_dir().join("runtime.json")
}
fn live_glb_path() -> PathBuf {
    runtime_dir().join("scene.glb")
}
fn camera_path() -> PathBuf {
    runtime_dir().join("camera.json")
}
fn read_runtime() -> Runtime {
    fs::read_to_string(runtime_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(tmp, path)?;
    Ok(())
}
fn write_runtime(runtime: &Runtime) -> Result<()> {
    write_atomic(&runtime_path(), &serde_json::to_vec_pretty(runtime)?)
}
fn absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}
fn parse(path: &Path) -> Result<Scene> {
    let scene: Scene = serde_json::from_str(
        &fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?,
    )
    .context("invalid scene JSON")?;
    for object in &scene.objects {
        if !matches!(
            object.kind.as_str(),
            "sphere" | "cube" | "cylinder" | "cone" | "line" | "arrow"
        ) {
            bail!("unsupported primitive: {}", object.kind);
        }
    }
    Ok(scene)
}
fn python() -> PathBuf {
    Path::new(MANIFEST).join(".venv/bin/python")
}
fn build_to(scene: &Path, output: &Path) -> Result<()> {
    parse(scene)?;
    let python = python();
    if !python.exists() {
        bail!("missing .venv; run scripts/setup_rasterminal.sh first");
    }
    let script = Path::new(MANIFEST).join("scripts/scene_to_glb.py");
    let tmp = output.with_extension("new.glb");
    let status = Command::new(python)
        .args([script.as_os_str(), scene.as_os_str(), tmp.as_os_str()])
        .status()?;
    if !status.success() {
        bail!("scene compiler failed");
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(tmp, output)?;
    Ok(())
}
fn stage(scene: &Path) -> Result<Runtime> {
    let scene = absolute(scene)?;
    let glb = live_glb_path();
    build_to(&scene, &glb)?;
    let mut runtime = read_runtime();
    runtime.scene = Some(scene.display().to_string());
    runtime.glb = Some(glb.display().to_string());
    runtime.camera_state = Some(camera_path().display().to_string());
    write_runtime(&runtime)?;
    Ok(runtime)
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    if let Some(value) = value.get(key).and_then(Value::as_str) {
        return Some(value.to_string());
    }
    match value {
        Value::Object(map) => map.values().find_map(|value| json_string(value, key)),
        Value::Array(values) => values.iter().find_map(|value| json_string(value, key)),
        _ => None,
    }
}

fn split_pane() -> Result<String> {
    let output = Command::new("herdr")
        .args([
            "pane",
            "split",
            "--current",
            "--direction",
            "right",
            "--cwd",
            MANIFEST,
            "--no-focus",
        ])
        .output()?;
    if !output.status.success() {
        bail!(
            "herdr split failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let text = String::from_utf8(output.stdout)?;
    let value: Value = serde_json::from_str(text.trim()).context("invalid herdr split output")?;
    json_string(&value, "pane_id").context("herdr did not return pane_id")
}

fn rasterminal() -> PathBuf {
    std::env::var_os("AI3D_RASTERMINAL")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(MANIFEST).join("vendor/rasterminal/build/rasterminal"))
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn viewer_is_alive(pid: u32) -> bool {
    Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            let command = String::from_utf8_lossy(&output.stdout);
            command.contains("ai3d") && command.contains("__viewer")
        })
        .unwrap_or(false)
}

fn spawn_renderer(glb: &Path, camera: &Path) -> Result<Child> {
    let binary = rasterminal();
    if !binary.exists() {
        bail!("rasterminal is not built; run scripts/setup_rasterminal.sh");
    }
    Ok(Command::new(binary)
        .arg(glb)
        .env("AI3D_WATCH", glb)
        .env("AI3D_STATE", camera)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?)
}

fn viewer() -> Result<()> {
    let mut runtime = read_runtime();
    let glb = runtime
        .glb
        .as_deref()
        .map(PathBuf::from)
        .context("no staged scene; run ai3d open/show first")?;
    let camera = camera_path();
    let mut child = spawn_renderer(&glb, &camera)?;
    runtime.viewer_pid = Some(std::process::id());
    write_runtime(&runtime)?;

    let mut watched_scene = PathBuf::new();
    let mut watched_mtime = None;
    loop {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                bail!("rasterminal exited with {status}");
            }
            break;
        }
        let current = read_runtime();
        if let Some(scene) = current.scene.as_deref().map(PathBuf::from) {
            let mtime = modified(&scene);
            if scene != watched_scene || (mtime.is_some() && mtime != watched_mtime) {
                match build_to(&scene, &glb) {
                    Ok(()) => {
                        watched_scene = scene;
                        watched_mtime = mtime;
                        eprintln!("ai3d: scene reloaded");
                    }
                    Err(error) => eprintln!("ai3d: reload failed: {error:#}"),
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    let mut runtime = read_runtime();
    runtime.viewer_pid = None;
    write_runtime(&runtime)?;
    Ok(())
}

fn state() -> Result<()> {
    let runtime = read_runtime();
    let mut scene = if let Some(path) = runtime.scene.as_deref() {
        serde_json::from_str::<Value>(&fs::read_to_string(path)?)?
    } else {
        json!({"objects": [], "arrows": []})
    };
    let camera = runtime
        .camera_state
        .as_deref()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.get("camera").cloned())
        .unwrap_or(Value::Null);
    scene["camera"] = camera;
    scene["runtime"] = json!({
        "pane": runtime.pane,
        "viewer_pid": runtime.viewer_pid,
        "glb": runtime.glb,
    });
    println!("{}", serde_json::to_string_pretty(&scene)?);
    Ok(())
}

fn close() -> Result<()> {
    let runtime = read_runtime();
    if let Some(pane) = runtime.pane.as_deref() {
        let status = Command::new("herdr")
            .args(["pane", "close", pane])
            .status()?;
        if !status.success() {
            bail!("failed to close ai3d pane {pane}");
        }
    } else if let Some(pid) = runtime.viewer_pid {
        if viewer_is_alive(pid) {
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status();
        }
    }
    write_runtime(&Runtime::default())?;
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Cmd::Build { scene } => {
            let output = absolute(&scene)?.with_extension("glb");
            build_to(&scene, &output)?;
            println!("{}", output.display());
            Ok(())
        }
        Cmd::Show { scene } => {
            let active = read_runtime().viewer_pid.is_some_and(viewer_is_alive);
            if !active {
                bail!("no active viewer; run ai3d open first");
            }
            let runtime = stage(&scene)?;
            println!("{}", serde_json::to_string_pretty(&runtime)?);
            Ok(())
        }
        Cmd::Open { scene } => {
            let existing = read_runtime();
            if existing.viewer_pid.is_some_and(viewer_is_alive) {
                let runtime = stage(&scene)?;
                println!("{}", serde_json::to_string_pretty(&runtime)?);
                return Ok(());
            }
            if let Some(stale_pane) = existing.pane.as_deref() {
                let _ = Command::new("herdr")
                    .args(["pane", "close", stale_pane])
                    .status();
            }
            let mut runtime = stage(&scene)?;
            if std::env::var_os("HERDR_ENV").is_none() {
                eprintln!("Herdr is unavailable. In a Ghostty right pane run:");
                eprintln!("{} __viewer", std::env::current_exe()?.display());
                return Ok(());
            }
            let pane = split_pane()?;
            runtime.pane = Some(pane.clone());
            write_runtime(&runtime)?;
            let exe = std::env::current_exe()?;
            let status = Command::new("herdr")
                .args(["pane", "run", &pane])
                .arg(exe)
                .arg("__viewer")
                .status()?;
            if !status.success() {
                let _ = Command::new("herdr")
                    .args(["pane", "close", &pane])
                    .status();
                bail!("failed to launch viewer in pane {pane}");
            }
            println!("{}", serde_json::to_string_pretty(&runtime)?);
            Ok(())
        }
        Cmd::State => state(),
        Cmd::Close => close(),
        Cmd::Screenshot => {
            println!(
                "{}",
                json!({
                    "supported": false,
                    "phase": 2,
                    "reason": "Ghostty pane pixel capture is unavailable; Kitty graphics are not present in PTY text"
                })
            );
            Ok(())
        }
        Cmd::Viewer => viewer(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_defaults() {
        let scene: Scene =
            serde_json::from_str(r#"{"objects":[{"id":"a","type":"sphere"}]}"#).unwrap();
        assert_eq!(scene.objects[0].scale, [1., 1., 1.]);
        assert_eq!(scene.objects[0].opacity, 1.);
    }

    #[test]
    fn rejects_unknown_primitive() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), r#"{"objects":[{"id":"a","type":"torus"}]}"#).unwrap();
        assert!(parse(file.path()).is_err());
    }
}
