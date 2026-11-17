//! Python Sidecar Management
//!
//! Bundles and manages Python Playwright scrapers as Tauri sidecar.

use anyhow::Result;
use std::path::PathBuf;
use tauri::AppHandle;
use tracing::info;

const SIDECAR_DIR: &str = "python-scrapers";
const SIDECAR_SCRIPT: &str = "main.py";

pub struct PythonSidecar {
    script_path: PathBuf,
    venv_path: Option<PathBuf>,
}

impl PythonSidecar {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let resource_dir = app_handle
            .path_resolver()
            .resource_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not resolve resource dir"))?;

        let script_path = resource_dir.join(SIDECAR_DIR).join(SIDECAR_SCRIPT);
        let venv_path = resource_dir.join(SIDECAR_DIR).join(".venv");

        Ok(Self {
            script_path,
            venv_path: if venv_path.exists() { Some(venv_path) } else { None },
        })
    }

    /// Create a PythonSidecar for development/testing with a custom script path.
    /// Uses the system Python interpreter (no venv).
    pub fn from_script_path(script_path: impl Into<PathBuf>) -> Self {
        Self {
            script_path: script_path.into(),
            venv_path: None,
        }
    }

    pub fn python_executable(&self) -> PathBuf {
        if let Some(venv) = &self.venv_path {
            #[cfg(target_os = "windows")]
            return venv.join("Scripts").join("python.exe");
            #[cfg(not(target_os = "windows"))]
            return venv.join("bin").join("python3");
        }
        // Fallback to system python
        #[cfg(target_os = "windows")]
        return PathBuf::from("python.exe");
        #[cfg(not(target_os = "windows"))]
        return PathBuf::from("python3");
    }

    pub async fn run_command(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        use tokio::process::Command;
        use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};

        let python = self.python_executable();
        let mut child = Command::new(&python)
            .arg(&self.script_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Send input
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(serde_json::to_string(&input)?.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }

        // Read output
        let stdout = child.stdout.take().expect("stdout not captured");
        let mut reader = BufReader::new(stdout).lines();
        let mut response = String::new();
        while let Some(line) = reader.next_line().await? {
            response = line;
            break; // First line is JSON response
        }

        // Wait for completion
        let status = child.wait().await?;
        if !status.success() {
            let stderr = child.stderr.take().expect("stderr not captured");
            let mut stderr_reader = BufReader::new(stderr).lines();
            let mut errors = Vec::new();
            while let Some(line) = stderr_reader.next_line().await? {
                errors.push(line);
            }
            return Err(anyhow::anyhow!("Sidecar failed: {}", errors.join("\n")));
        }

        Ok(serde_json::from_str(&response)?)
    }
}

pub fn ensure_python_sidecar(app_handle: &AppHandle) -> Result<()> {
    let resource_dir = app_handle
        .path_resolver()
        .resource_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not resolve resource dir"))?;

    let sidecar_dir = resource_dir.join(SIDECAR_DIR);
    let requirements = sidecar_dir.join("requirements.txt");

    if !sidecar_dir.exists() {
        info!("Python sidecar not found at {:?}, will be created on build", sidecar_dir);
        return Ok(()); // Will be created during build
    }

    // Check if venv exists and has requirements
    let venv_dir = sidecar_dir.join(".venv");
    if !venv_dir.exists() {
        info!("Creating Python virtual environment for sidecar");
        create_venv(&sidecar_dir)?;
    }

    // Install/update requirements
    if requirements.exists() {
        install_requirements(&sidecar_dir)?;
    }

    Ok(())
}

fn create_venv(sidecar_dir: &PathBuf) -> Result<()> {
    use std::process::Command;

    let python = if cfg!(target_os = "windows") { "python.exe" } else { "python3" };
    let output = Command::new(python)
        .args(["-m", "venv", ".venv"])
        .current_dir(sidecar_dir)
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to create venv: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(())
}

fn install_requirements(sidecar_dir: &PathBuf) -> Result<()> {
    use std::process::Command;

    let python = sidecar_dir.join(".venv").join(if cfg!(target_os = "windows") { "Scripts/python.exe" } else { "bin/python3" });
    let output = Command::new(&python)
        .args(["-m", "pip", "install", "-r", "requirements.txt"])
        .current_dir(sidecar_dir)
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to install requirements: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(())
}