use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context, Result, bail};
use async_fs as fs;
use clap::Parser;
use console::style;

/// Build a Tauri application
#[derive(Debug, Clone, Parser)]
pub struct TauriCommand {
    #[clap(subcommand)]
    pub subcommand: TauriSubcommand,
}

#[derive(Debug, Clone, Parser)]
pub enum TauriSubcommand {
    /// Build a Tauri app from a Luau script
    Build(TauriBuildCommand),
}

/// Build a Tauri application from a Luau script
#[derive(Debug, Clone, Parser)]
pub struct TauriBuildCommand {
    /// The path to the input Luau script
    pub input: PathBuf,

    /// The path to the output executable
    #[clap(short, long)]
    pub output: Option<PathBuf>,
}

impl TauriCommand {
    pub async fn run(self) -> Result<ExitCode> {
        match self.subcommand {
            TauriSubcommand::Build(cmd) => cmd.run().await,
        }
    }
}

impl TauriBuildCommand {
    pub async fn run(self) -> Result<ExitCode> {
        println!(
            "{} Tauri build from {}",
            style("Starting").cyan().bold(),
            style(self.input.display()).green()
        );

        // 1. Read and parse the input script to extract tauri.new() config
        let script_content = fs::read_to_string(&self.input)
            .await
            .context("Failed to read input script")?;

        // Parse tauri.new() call to extract config
        let config = parse_tauri_config(&script_content)?;

        println!(
            "  {} App: {} ({})",
            style("→").dim(),
            style(&config.name).yellow(),
            &config.identifier
        );

        // 2. Create temp directory for the Tauri project
        let temp_dir = std::env::temp_dir().join(format!("lune-tauri-{}", uuid_simple()));
        fs::create_dir_all(&temp_dir).await?;

        println!(
            "  {} Building in {}",
            style("→").dim(),
            style(temp_dir.display()).dim()
        );

        // 3. Generate Tauri project files
        generate_cargo_toml(&temp_dir, &config).await?;
        generate_main_rs(&temp_dir, &script_content).await?;
        generate_tauri_conf(&temp_dir, &config).await?;
        generate_capabilities(&temp_dir).await?;
        generate_icons(&temp_dir).await?;

        // 4. Copy HTML/assets if specified
        if let Some(ref html_path) = config.html {
            let html_src = self
                .input
                .parent()
                .unwrap_or(Path::new("."))
                .join(html_path);
            let dist_dir = temp_dir.join("dist");
            fs::create_dir_all(&dist_dir).await?;

            if html_src.is_dir() {
                copy_dir_recursive_sync(&html_src, &dist_dir)?;
            } else {
                let dest = dist_dir.join("index.html");
                fs::copy(&html_src, &dest).await?;
            }
        } else {
            // Create default index.html
            let dist_dir = temp_dir.join("dist");
            fs::create_dir_all(&dist_dir).await?;
            fs::write(dist_dir.join("index.html"), DEFAULT_HTML).await?;
        }

        // 5. Run cargo build
        println!(
            "  {} Running cargo build (this may take a few minutes)...",
            style("→").dim()
        );

        let status = std::process::Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&temp_dir)
            .status()
            .context("Failed to run cargo build")?;

        if !status.success() {
            bail!("Cargo build failed with exit code: {:?}", status.code());
        }

        // 6. Copy output binary
        let built_exe = temp_dir.join("target").join("release").join(format!(
            "{}.exe",
            config.name.replace(" ", "-").to_lowercase()
        ));

        let output_path = self.output.clone().unwrap_or_else(|| {
            let name = config.name.replace(" ", "-").to_lowercase();
            PathBuf::from(format!("{}.exe", name))
        });

        fs::copy(&built_exe, &output_path)
            .await
            .context("Failed to copy output binary")?;

        println!(
            "{} Built successfully: {}",
            style("✓").green().bold(),
            style(output_path.display()).blue()
        );

        Ok(ExitCode::SUCCESS)
    }
}

#[derive(Debug, Default)]
struct TauriConfig {
    name: String,
    identifier: String,
    version: String,
    icon: Option<String>,
    html: Option<String>,
    window_title: String,
    window_width: u32,
    window_height: u32,
}

fn parse_tauri_config(script: &str) -> Result<TauriConfig> {
    // Simple regex-based extraction of tauri.new({...}) config
    // This is a basic implementation - a full parser would be more robust

    let mut config = TauriConfig {
        name: "Lune App".to_string(),
        identifier: "org.lune.app".to_string(),
        version: "0.1.0".to_string(),
        window_title: "Lune App".to_string(),
        window_width: 800,
        window_height: 600,
        ..Default::default()
    };

    // Extract name
    if let Some(cap) = regex_find(script, r#"name\s*=\s*"([^"]+)""#) {
        config.name = cap;
        config.window_title = config.name.clone();
    }

    // Extract identifier
    if let Some(cap) = regex_find(script, r#"identifier\s*=\s*"([^"]+)""#) {
        config.identifier = cap;
    }

    // Extract version
    if let Some(cap) = regex_find(script, r#"version\s*=\s*"([^"]+)""#) {
        config.version = cap;
    }

    // Extract html
    if let Some(cap) = regex_find(script, r#"html\s*=\s*"([^"]+)""#) {
        config.html = Some(cap);
    }

    // Extract icon
    if let Some(cap) = regex_find(script, r#"icon\s*=\s*"([^"]+)""#) {
        config.icon = Some(cap);
    }

    // Extract window config
    if let Some(cap) = regex_find(script, r#"title\s*=\s*"([^"]+)""#) {
        config.window_title = cap;
    }
    if let Some(cap) = regex_find(script, r#"width\s*=\s*(\d+)"#) {
        config.window_width = cap.parse().unwrap_or(800);
    }
    if let Some(cap) = regex_find(script, r#"height\s*=\s*(\d+)"#) {
        config.window_height = cap.parse().unwrap_or(600);
    }

    Ok(config)
}

fn regex_find(text: &str, pattern: &str) -> Option<String> {
    let re = regex::Regex::new(pattern).ok()?;
    re.captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:x}{:x}", dur.as_secs(), dur.subsec_nanos())
}

async fn generate_cargo_toml(dir: &Path, config: &TauriConfig) -> Result<()> {
    let content = format!(
        r#"[package]
name = "{}"
version = "{}"
edition = "2021"

[dependencies]
tauri = {{ version = "2", features = [] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"

[build-dependencies]
tauri-build = {{ version = "2", features = [] }}
"#,
        config.name.replace(" ", "-").to_lowercase(),
        config.version
    );
    fs::write(dir.join("Cargo.toml"), content).await?;

    // build.rs
    fs::write(dir.join("build.rs"), "fn main() { tauri_build::build() }").await?;

    Ok(())
}

async fn generate_main_rs(dir: &Path, script: &str) -> Result<()> {
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).await?;

    // For now, generate a simple Tauri app without embedded Lune
    // The full implementation would embed the Lune runtime
    let content = r#"#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
"#;
    fs::write(src_dir.join("main.rs"), content).await?;

    // Save script for future embedding
    fs::write(dir.join("script.luau"), script).await?;

    Ok(())
}

async fn generate_tauri_conf(dir: &Path, config: &TauriConfig) -> Result<()> {
    let content = format!(
        r#"{{
    "productName": "{}",
    "version": "{}",
    "identifier": "{}",
    "build": {{
        "frontendDist": "./dist"
    }},
    "bundle": {{
        "active": false
    }},
    "app": {{
        "withGlobalTauri": true,
        "windows": [
            {{
                "title": "{}",
                "width": {},
                "height": {},
                "resizable": true
            }}
        ],
        "security": {{
            "csp": null
        }}
    }}
}}"#,
        config.name,
        config.version,
        config.identifier,
        config.window_title,
        config.window_width,
        config.window_height
    );
    fs::write(dir.join("tauri.conf.json"), content).await?;
    Ok(())
}

async fn generate_capabilities(dir: &Path) -> Result<()> {
    let cap_dir = dir.join("capabilities");
    fs::create_dir_all(&cap_dir).await?;

    let content = r#"{
    "identifier": "default",
    "description": "Default capability",
    "windows": ["*"],
    "permissions": [
        "core:event:default",
        "core:window:default"
    ]
}"#;
    fs::write(cap_dir.join("default.json"), content).await?;
    Ok(())
}

async fn generate_icons(dir: &Path) -> Result<()> {
    let icons_dir = dir.join("icons");
    fs::create_dir_all(&icons_dir).await?;

    // Minimal valid ICO: create a simple 16x16 32-bit icon
    let mut ico = Vec::new();
    // ICO Header
    ico.extend_from_slice(&[0x00, 0x00]); // Reserved
    ico.extend_from_slice(&[0x01, 0x00]); // Type: ICO
    ico.extend_from_slice(&[0x01, 0x00]); // Count: 1
    // ICONDIRENTRY
    ico.push(0x10); // Width: 16
    ico.push(0x10); // Height: 16
    ico.push(0x00); // Colors
    ico.push(0x00); // Reserved
    ico.extend_from_slice(&[0x01, 0x00]); // Planes
    ico.extend_from_slice(&[0x20, 0x00]); // Bits: 32
    let img_size: u32 = 40 + (16 * 16 * 4) + (16 * 4); // header + pixels + mask
    ico.extend_from_slice(&img_size.to_le_bytes());
    ico.extend_from_slice(&[0x16, 0x00, 0x00, 0x00]); // Offset: 22
    // BITMAPINFOHEADER
    ico.extend_from_slice(&[0x28, 0x00, 0x00, 0x00]); // Size: 40
    ico.extend_from_slice(&[0x10, 0x00, 0x00, 0x00]); // Width: 16
    ico.extend_from_slice(&[0x20, 0x00, 0x00, 0x00]); // Height: 32
    ico.extend_from_slice(&[0x01, 0x00]); // Planes
    ico.extend_from_slice(&[0x20, 0x00]); // Bits: 32
    ico.extend_from_slice(&[0x00; 24]); // Rest of header
    // Pixel data: 16x16 BGRA (blue square)
    for _ in 0..(16 * 16) {
        ico.extend_from_slice(&[0xFF, 0x80, 0x00, 0xFF]); // Blue
    }
    // AND mask
    ico.extend_from_slice(&[0x00; 64]);

    fs::write(icons_dir.join("icon.ico"), ico).await?;
    Ok(())
}

fn copy_dir_recursive_sync(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_dir_recursive_sync(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

const DEFAULT_HTML: &str = r#"<!doctype html>
<html>
<head>
    <style>
        body { font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #f0f0f0; }
        h1 { color: #333; }
    </style>
</head>
<body>
    <h1>Hello from Lune!</h1>
</body>
</html>
"#;
