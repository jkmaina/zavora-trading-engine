// File: tests/test_helpers.rs

use std::process::{Command, Child};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

// Singleton to manage the engine process
lazy_static::lazy_static! {
    static ref ENGINE_PROCESS: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
}

// Start the trading engine for tests
pub fn start_engine() -> Result<(), String> {
    let mut process_guard = ENGINE_PROCESS.lock().unwrap();
    
    // If the engine is already running, do nothing
    if process_guard.is_some() {
        return Ok(());
    }
    
    // Start the "trading-engine" binary which should be the main executable for the workspace
    let process = Command::new("cargo")
        .args(["run", "--bin", "trading-engine"])
        .spawn()
        .map_err(|e| format!("Failed to start engine: {}", e))?;
    
    *process_guard = Some(process);
    
    // Give the engine some time to start up
    drop(process_guard);
    thread::sleep(Duration::from_secs(2));
    
    // Simple health check to verify the engine is running
    let health_check = Command::new("curl")
        .args(["-s", "http://localhost:8080/health"])
        .output()
        .map_err(|e| format!("Health check failed: {}", e))?;
    
    if !health_check.status.success() {
        stop_engine()?;
        return Err("Engine failed to start properly".to_string());
    }
    
    Ok(())
}

// Stop the trading engine after tests
pub fn stop_engine() -> Result<(), String> {
    let mut process_guard = ENGINE_PROCESS.lock().unwrap();
    
    if let Some(mut child) = process_guard.take() {
        child.kill()
            .map_err(|e| format!("Failed to kill engine process: {}", e))?;
        
        child.wait()
            .map_err(|e| format!("Failed to wait for engine process: {}", e))?;
    }
    
    Ok(())
}

// Automatically start and stop the engine for a test
pub struct EngineGuard;

impl EngineGuard {
    pub fn new() -> Result<Self, String> {
        start_engine()?;
        Ok(Self)
    }
}

impl Drop for EngineGuard {
    fn drop(&mut self) {
        let _ = stop_engine();
    }
}