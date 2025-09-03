//! Integration tests to ensure the application builds and basic functionality works

use std::process::Command;

#[test]
fn test_binary_builds_and_runs() {
    // Test that cargo build succeeds
    let build_output = Command::new("cargo")
        .args(&["build", "--release"])
        .output()
        .expect("Failed to execute cargo build");
    
    assert!(build_output.status.success(), "Cargo build failed: {}", 
            String::from_utf8_lossy(&build_output.stderr));
    
    // Test that the binary exists and can show help
    let help_output = Command::new("./target/release/VideoReencodingNet")
        .args(&["--help"])
        .output()
        .expect("Failed to execute binary");
    
    assert!(help_output.status.success(), "Binary help command failed");
    
    // Check that help output contains expected content
    let help_text = String::from_utf8_lossy(&help_output.stdout);
    assert!(help_text.contains("VideoReencodingNet"), "Help text doesn't contain application name");
}