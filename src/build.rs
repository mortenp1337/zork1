//! Zork I Build Tool
//! 
//! This tool compiles the ZIL source files into a Z-Machine story file.
//! It uses ZILF (a .NET-based ZIL compiler) to perform the compilation.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

fn main() {
    println!("Zork I Build Tool");
    println!("=================");
    println!();
    
    // Get the project directory
    let project_dir = get_project_dir();
    println!("Project directory: {}", project_dir.display());
    
    // Check for ZILF
    let zilf_path = find_zilf();
    let zapf_path = find_zapf();
    
    match (&zilf_path, &zapf_path) {
        (Some(zilf), Some(zapf)) => {
            compile_with_zilf(&project_dir, zilf, zapf);
        }
        _ => {
            println!();
            println!("ZILF compiler not found. Please install ZILF first.");
            println!();
            println!("To install ZILF:");
            println!("  1. Install .NET SDK 9.0 or later");
            println!("  2. Download ZILF from: https://github.com/taradinoc/zilf");
            println!("  3. Build ZILF with: dotnet build Zilf.sln");
            println!("  4. Add zilf and zapf to your PATH");
            println!();
            println!("Alternatively, the precompiled game is available at:");
            println!("  COMPILED/zork1.z3");
            exit(1);
        }
    }
}

fn get_project_dir() -> PathBuf {
    // Try to find the project directory containing the ZIL files
    let cwd = env::current_dir().expect("Failed to get current directory");
    
    // Check if ZIL files exist in current directory
    if cwd.join("zork1.zil").exists() {
        return cwd;
    }
    
    // Check parent directories
    let mut dir = cwd.clone();
    for _ in 0..3 {
        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
            if dir.join("zork1.zil").exists() {
                return dir;
            }
        }
    }
    
    cwd
}

fn find_zilf() -> Option<PathBuf> {
    // Try to find zilf in PATH
    if let Ok(output) = Command::new("which").arg("zilf").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    
    // Try dotnet tool
    if let Ok(output) = Command::new("dotnet").args(["tool", "list", "-g"]).output() {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("zilf") {
                return Some(PathBuf::from("zilf"));
            }
        }
    }
    
    // Check common installation locations
    let home = env::var("HOME").unwrap_or_default();
    let paths = [
        format!("{}/.dotnet/tools/zilf", home),
        "/usr/local/bin/zilf".to_string(),
        "/usr/bin/zilf".to_string(),
    ];
    
    for path in paths {
        if Path::new(&path).exists() {
            return Some(PathBuf::from(path));
        }
    }
    
    None
}

fn find_zapf() -> Option<PathBuf> {
    // Try to find zapf in PATH
    if let Ok(output) = Command::new("which").arg("zapf").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    
    // Try dotnet tool
    if let Ok(output) = Command::new("dotnet").args(["tool", "list", "-g"]).output() {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("zapf") {
                return Some(PathBuf::from("zapf"));
            }
        }
    }
    
    // Check common installation locations
    let home = env::var("HOME").unwrap_or_default();
    let paths = [
        format!("{}/.dotnet/tools/zapf", home),
        "/usr/local/bin/zapf".to_string(),
        "/usr/bin/zapf".to_string(),
    ];
    
    for path in paths {
        if Path::new(&path).exists() {
            return Some(PathBuf::from(path));
        }
    }
    
    None
}

fn compile_with_zilf(project_dir: &Path, zilf: &Path, zapf: &Path) {
    let zil_file = project_dir.join("zork1.zil");
    let zap_file = project_dir.join("zork1.zap");
    let z3_file = project_dir.join("target").join("zork1.z3");
    
    // Create target directory
    let target_dir = project_dir.join("target");
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir).expect("Failed to create target directory");
    }
    
    println!("Compiling ZIL sources...");
    println!("  Source: {}", zil_file.display());
    
    // Run ZILF to compile ZIL to ZAP
    let zilf_status = Command::new(zilf)
        .arg(&zil_file)
        .current_dir(project_dir)
        .status();
    
    match zilf_status {
        Ok(status) if status.success() => {
            println!("  ZIL compilation successful.");
        }
        Ok(status) => {
            eprintln!("  ZILF failed with exit code: {:?}", status.code());
            exit(1);
        }
        Err(e) => {
            eprintln!("  Failed to run ZILF: {}", e);
            exit(1);
        }
    }
    
    // Check if ZAP file was created
    if !zap_file.exists() {
        eprintln!("  Error: ZAP file was not created.");
        exit(1);
    }
    
    println!("Assembling Z-Machine code...");
    
    // Run ZAPF to assemble ZAP to Z3
    let zapf_status = Command::new(zapf)
        .arg(&zap_file)
        .arg("-o")
        .arg(&z3_file)
        .current_dir(project_dir)
        .status();
    
    match zapf_status {
        Ok(status) if status.success() => {
            println!("  Assembly successful.");
        }
        Ok(status) => {
            eprintln!("  ZAPF failed with exit code: {:?}", status.code());
            exit(1);
        }
        Err(e) => {
            eprintln!("  Failed to run ZAPF: {}", e);
            exit(1);
        }
    }
    
    // Verify output
    if z3_file.exists() {
        let metadata = fs::metadata(&z3_file).expect("Failed to get file metadata");
        println!();
        println!("Build successful!");
        println!("  Output: {}", z3_file.display());
        println!("  Size: {} bytes", metadata.len());
        println!();
        println!("Run the game with: cargo run --bin zork1");
    } else {
        eprintln!();
        eprintln!("Error: Output file was not created.");
        exit(1);
    }
}
