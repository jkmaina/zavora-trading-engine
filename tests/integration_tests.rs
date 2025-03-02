// File: tests/integration_tests.rs

mod test_helpers;
use test_helpers::EngineGuard;
use std::process::Command;
use std::path::Path;

// Helper function to run shell scripts
fn run_shell_script(script_path: &str) -> Result<(), String> {
    let output = Command::new("sh")
        .arg(script_path)
        .output()
        .map_err(|e| format!("Failed to execute script: {}", e))?;
    
    if !output.status.success() {
        return Err(format!(
            "Script execution failed: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        ));
    }
    
    println!("Script output: {}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

// Helper function to run Node.js scripts
fn run_node_script(script_path: &str) -> Result<(), String> {
    let output = Command::new("node")
        .arg(script_path)
        .output()
        .map_err(|e| format!("Failed to execute Node.js script: {}", e))?;
    
    if !output.status.success() {
        return Err(format!(
            "Node.js script execution failed: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        ));
    }
    
    println!("Script output: {}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

#[test]
#[ignore = "This requires PostgreSQL client to be installed"]
fn test_api() {
    // Start the engine and ensure it gets stopped when the test ends
    let _guard = EngineGuard::new().expect("Failed to start trading engine");
    
    let script_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("api_test.sh");
    run_shell_script(script_path.to_str().unwrap()).expect("API test failed");
}

#[test]
fn test_websocket() {
    // Start the engine and ensure it gets stopped when the test ends
    let _guard = EngineGuard::new().expect("Failed to start trading engine");
    
    let script_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ws_test.js");
    run_node_script(script_path.to_str().unwrap()).expect("WebSocket test failed");
}