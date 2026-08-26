use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use notify::{RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}
#[derive(Subcommand)]
enum Cmd {
    Build { scene: PathBuf },
    Show { scene: PathBuf },
    Open { scene: PathBuf },
    State,
    Close,
    Screenshot,
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
#[derive(Serialize, Deserialize, Default)]
struct Runtime {
    scene: Option<String>,
    glb: Option<String>,
    pane: Option<String>,
}
fn runtime_path() -> PathBuf {
    dirs::runtime_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("ai3d-runtime.json")
}
fn read_runtime() -> Runtime {
    fs::read_to_string(runtime_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
fn write_runtime(r: &Runtime) -> Result<()> {
    fs::write(runtime_path(), serde_json::to_vec_pretty(r)?)?;
    Ok(())
}
fn parse(p: &Path) -> Result<Scene> {
    serde_json::from_str(&fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?)
        .context("invalid scene JSON")
}
fn color(s: &str) -> [f32; 4] {
    let n = s.trim_start_matches('#');
    if n.len() >= 6 {
        let x = |i| u8::from_str_radix(&n[i..i + 2], 16).unwrap_or(255) as f32 / 255.;
        [x(0), x(2), x(4), 1.]
    } else {
        match s {
            "red" => [1., 0., 0., 1.],
            "blue" => [0., 0., 1., 1.],
            "yellow" => [1., 1., 0., 1.],
            "green" => [0., 1., 0., 1.],
            _ => [0.7, 0.7, 0.7, 1.],
        }
    }
}
fn build(scene: &Path) -> Result<PathBuf> {
    let s = parse(scene)?;
    let out = scene.with_extension("glb");
    let mut meshes = Vec::new();
    for o in &s.objects {
        let c = color(&o.color);
        meshes.push(format!(
            r#"{{"primitives":[{{"attributes":{{"POSITION":0}},"material":{}}}]}}"#,
            meshes.len()
        ));
        let _ = c;
    }
    let json = format!(
        r#"{{"asset":{{"version":"2.0","generator":"ai3d"}},"scene":0,"scenes":[{{"nodes":[{}]}}],"nodes":[{}],"meshes":[{}],"materials":[{{"pbrMetallicRoughness":{{"baseColorFactor":[1,0,0,1]}}}}],"buffers":[{{"byteLength":0}}]}}"#,
        (0..s.objects.len())
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(","),
        (0..s.objects.len())
            .map(|i| format!(r#"{{"mesh":{}}}"#, i))
            .collect::<Vec<_>>()
            .join(","),
        meshes.join(",")
    );
    let mut bytes = b"glTF".to_vec();
    bytes.extend_from_slice(&2u32.to_le_bytes());
    let total = 12 + 8 + json.len();
    bytes.extend_from_slice(&(total as u32).to_le_bytes());
    bytes.extend_from_slice(&(json.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b"JSON");
    bytes.extend_from_slice(json.as_bytes());
    fs::write(&out, bytes)?;
    Ok(out)
}
fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Build { scene } => {
            println!("{}", build(&scene)?.display());
            Ok(())
        }
        Cmd::State => {
            let r = read_runtime();
            if let Some(p) = r.scene {
                println!("{}", fs::read_to_string(p)?);
            } else {
                println!(r#"{{"camera":null,"objects":[]}}"#);
            }
            Ok(())
        }
        Cmd::Show { scene } => {
            let glb = build(&scene)?;
            let mut r = read_runtime();
            r.scene = Some(scene.display().to_string());
            r.glb = Some(glb.display().to_string());
            write_runtime(&r)?;
            watch(scene)?;
            Ok(())
        }
        Cmd::Open { scene } => {
            let glb = build(&scene)?;
            let mut r = read_runtime();
            r.scene = Some(scene.display().to_string());
            r.glb = Some(glb.display().to_string());
            if std::env::var("HERDR_ENV").is_ok() {
                let out = Command::new("herdr")
                    .args([
                        "pane",
                        "split",
                        "--current",
                        "--direction",
                        "right",
                        "--no-focus",
                    ])
                    .output()?;
                r.pane = Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
            } else {
                eprintln!(
                    "HERDR_ENV unavailable; run rasterminal {} in a right pane",
                    glb.display()
                );
            }
            write_runtime(&r)?;
            watch(scene)?;
            Ok(())
        }
        Cmd::Close => {
            let mut r = read_runtime();
            if let Some(p) = r.pane {
                let _ = Command::new("herdr")
                    .args(["pane", "close", "--pane", &p])
                    .status();
            }
            write_runtime(&Runtime::default())?;
            Ok(())
        }
        Cmd::Screenshot => {
            println!(
                r#"{{"supported":false,"reason":"Ghostty pane pixel capture is unavailable; Kitty graphics are not in PTY text."}}"#
            );
            Ok(())
        }
    }
}
fn watch(scene: PathBuf) -> Result<()> {
    let _ = build(&scene)?;
    let (tx, rx) = std::sync::mpsc::channel();
    let mut w = notify::recommended_watcher(tx)?;
    w.watch(&scene, RecursiveMode::NonRecursive)?;
    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(_) => {
                let glb = build(&scene)?;
                let mut r = read_runtime();
                r.glb = Some(glb.display().to_string());
                write_runtime(&r)?;
                println!("reloaded {}", glb.display());
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses() {
        let s: Scene = serde_json::from_str(r#"{"objects":[{"id":"a","type":"sphere"}]}"#).unwrap();
        assert_eq!(s.objects[0].scale, [1., 1., 1.]);
    }
}
