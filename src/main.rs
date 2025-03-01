use std::env;
use std::process::Command;
use std::path::PathBuf;

fn main() {
    // This is a simple proxy to launch the trading-engine binary
    println!("Starting Zavora Trading Engine...");
    
    // Get current directory as a base path
    let current_dir = env::current_dir()
        .expect("Failed to get current directory");
    
    // Determine target directory and build profile
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    
    // First try to find the binary in the standard cargo target directory
    let mut binary_path = current_dir.join(format!("target/{}/trading-engine", profile));
    
    // If that doesn't exist, try looking in the workspace target directory
    if !binary_path.exists() {
        if let Ok(workspace_dir) = env::var("CARGO_WORKSPACE_DIR") {
            binary_path = PathBuf::from(workspace_dir).join(format!("target/{}/trading-engine", profile));
        }
    }
    
    // Add .exe extension on Windows
    #[cfg(target_os = "windows")]
    {
        binary_path.set_extension("exe");
    }
    
    println!("Launching: {:?}", binary_path);
    
    // Execute the actual trading engine binary
    let status = Command::new(&binary_path)
        .args(env::args().skip(1))
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Failed to execute trading-engine binary at {:?}: {}", binary_path, e);
            std::process::exit(1);
        });
    
    // Exit with the same status code
    std::process::exit(status.code().unwrap_or(1));
}