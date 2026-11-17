use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

fn main() {
    // Copy CV templates and fonts to output directory for runtime access
    copy_cv_resources();

    // Tell cargo to rerun if migrations change
    println!("cargo:rerun-if-changed=migrations/");

    // Tell cargo to rerun if sidecar source changes
    println!("cargo:rerun-if-changed=sidecar/python-scrapers/src/");
    println!("cargo:rerun-if-changed=sidecar/python-scrapers/requirements.txt");
    println!("cargo:rerun-if-changed=sidecar/python-scrapers/pyproject.toml");

    // Tell cargo to rerun if CV templates or fonts change
    println!("cargo:rerun-if-changed=templates/cv/");
    println!("cargo:rerun-if-changed=resources/fonts/");

    // SQLx compile-time query checking
    // Set DATABASE_URL for sqlx-cli at compile time
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        let app_dir = directories::ProjectDirs::from("com", "tunisiacareerscout", "app")
            .expect("Could not find app data directory");
        format!("sqlite://{}?mode=rwc", app_dir.data_dir().join("app.db").display())
    });
    env::set_var("DATABASE_URL", &database_url);

    // Build Python sidecar
    build_python_sidecar();

    // Embed frontend assets for production builds
    #[cfg(not(debug_assertions))]
    {
        let frontend_dist = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("../frontend/dist");
        if frontend_dist.exists() {
            println!("cargo:rustc-env=TAURI_FRONTEND_DIST={}", frontend_dist.display());
        }
    }

    // tauri_build::build()  // Temporarily disabled due to tauri.conf.json compatibility issue
}


fn build_python_sidecar() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let sidecar_dir = manifest_dir.join("sidecar/python-scrapers");
    let target_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("sidecar");
    let sidecar_name = if cfg!(target_os = "windows") {
        "python-scrapers.exe"
    } else {
        "python-scrapers"
    };
    let output_path = target_dir.join(sidecar_name);

    // Create target directory
    std::fs::create_dir_all(&target_dir).expect("Failed to create sidecar target directory");

    // Check if we should use PyInstaller or venv
    let use_pyinstaller = Command::new("pyinstaller")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if use_pyinstaller {
        println!("cargo:warning=Building Python sidecar with PyInstaller");
        build_with_pyinstaller(&sidecar_dir, &output_path, sidecar_name);
    } else {
        println!("cargo:warning=PyInstaller not found, building Python sidecar with venv");
        build_with_venv(&sidecar_dir, &target_dir, sidecar_name);
    }

    // Verify the sidecar was built
    if output_path.exists() {
        println!("cargo:warning=Python sidecar built at: {}", output_path.display());
        // Set env var for Tauri to find the sidecar
        println!("cargo:rustc-env=SIDECAR_PYTHON_SCRAPERS={}", output_path.display());
    } else {
        panic!("Failed to build Python sidecar at {}", output_path.display());
    }
}

fn build_with_pyinstaller(sidecar_dir: &Path, output_path: &Path, sidecar_name: &str) {
    let src_main = sidecar_dir.join("src/__main__.py");
    
    // PyInstaller command
    let mut cmd = Command::new("pyinstaller");
    cmd.current_dir(sidecar_dir)
        .args([
            "--onefile",
            "--name", "python-scrapers",
            "--distpath", output_path.parent().unwrap().to_str().unwrap(),
            "--workpath", sidecar_dir.join("build").to_str().unwrap(),
            "--specpath", sidecar_dir.join("build").to_str().unwrap(),
            "--clean",
            "--noconfirm",
            // Hidden imports for playwright
            "--hidden-import", "playwright",
            "--hidden-import", "playwright.async_api",
            "--hidden-import", "playwright._impl",
            "--hidden-import", "playwright_stealth",
            "--hidden-import", "bs4",
            "--hidden-import", "lxml",
            // Entry point
            src_main.to_str().unwrap(),
        ])
        .env("PYTHONPATH", sidecar_dir.join("src").to_str().unwrap());

    let status = cmd.status().expect("Failed to run pyinstaller");
    if !status.success() {
        panic!("PyInstaller build failed");
    }

    // PyInstaller puts the binary in dist/ subdirectory
    let pyinstaller_output = sidecar_dir.join("dist").join(sidecar_name);
    if pyinstaller_output.exists() {
        std::fs::copy(&pyinstaller_output, output_path).expect("Failed to copy sidecar binary");
    }
}

fn build_with_venv(sidecar_dir: &Path, target_dir: &Path, sidecar_name: &str) {
    let venv_dir = sidecar_dir.join(".venv");
    let python_exe = if cfg!(target_os = "windows") {
        venv_dir.join("Scripts/python.exe")
    } else {
        venv_dir.join("bin/python")
    };
    let pip_exe = if cfg!(target_os = "windows") {
        venv_dir.join("Scripts/pip.exe")
    } else {
        venv_dir.join("bin/pip")
    };

    // Create venv if it doesn't exist
    if !venv_dir.exists() {
        println!("cargo:warning=Creating Python virtual environment");
        let status = Command::new("python3")
            .args(["-m", "venv", venv_dir.to_str().unwrap()])
            .status()
            .expect("Failed to create venv");
        if !status.success() {
            panic!("Failed to create Python virtual environment");
        }
    }

    // Install dependencies
    println!("cargo:warning=Installing Python dependencies");
    let requirements = sidecar_dir.join("requirements.txt");
    let status = Command::new(pip_exe.to_str().unwrap())
        .args(["install", "-r", requirements.to_str().unwrap()])
        .status()
        .expect("Failed to install requirements");
    if !status.success() {
        panic!("Failed to install Python requirements");
    }

    // Install playwright browsers
    println!("cargo:warning=Installing Playwright browsers");
    let status = Command::new(python_exe.to_str().unwrap())
        .args(["-m", "playwright", "install", "chromium"])
        .status()
        .expect("Failed to install Playwright browsers");
    if !status.success() {
        println!("cargo:warning=Playwright browser install failed, continuing anyway");
    }

    // Create a wrapper script that activates venv and runs the module
    let wrapper_path = target_dir.join(sidecar_name);
    let src_dir = sidecar_dir.join("src");
    
    #[cfg(target_os = "windows")]
    {
        let wrapper_content = format!(
            "@echo off\n{} -m src %*\n",
            python_exe.to_str().unwrap().replace('/', "\\")
        );
        std::fs::write(&wrapper_path, wrapper_content).expect("Failed to write wrapper script");
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        let wrapper_content = format!(
            "#!/bin/bash\n{} -m src \"$@\"\n",
            python_exe.to_str().unwrap()
        );
        std::fs::write(&wrapper_path, wrapper_content).expect("Failed to write wrapper script");
        
        // Make executable
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&wrapper_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&wrapper_path, perms).unwrap();
    }

    println!("cargo:warning=Python sidecar wrapper created at: {}", wrapper_path.display());
}

fn copy_cv_resources() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Copy templates
    let templates_src = manifest_dir.join("templates/cv");
    let templates_dst = out_dir.join("templates/cv");
    if templates_src.exists() {
        fs::create_dir_all(&templates_dst).expect("Failed to create templates output directory");
        for entry in fs::read_dir(&templates_src).expect("Failed to read templates directory") {
            let entry = entry.expect("Failed to read directory entry");
            let src = entry.path();
            let dst = templates_dst.join(entry.file_name());
            fs::copy(&src, &dst).expect(&format!("Failed to copy template: {:?}", src));
            println!("cargo:warning=Copied template: {:?} -> {:?}", src, dst);
        }
    }
    
    // Copy fonts
    let fonts_src = manifest_dir.join("resources/fonts");
    let fonts_dst = out_dir.join("resources/fonts");
    if fonts_src.exists() {
        fs::create_dir_all(&fonts_dst).expect("Failed to create fonts output directory");
        for entry in fs::read_dir(&fonts_src).expect("Failed to read fonts directory") {
            let entry = entry.expect("Failed to read directory entry");
            let src = entry.path();
            let dst = fonts_dst.join(entry.file_name());
            fs::copy(&src, &dst).expect(&format!("Failed to copy font: {:?}", src));
            println!("cargo:warning=Copied font: {:?} -> {:?}", src, dst);
        }
    }
    
    // Set environment variables for runtime to find resources
    println!("cargo:rustc-env=CV_TEMPLATES_DIR={}", templates_dst.display());
    println!("cargo:rustc-env=CV_FONTS_DIR={}", fonts_dst.display());
}