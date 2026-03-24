use std::process::Command;
use std::fs;

#[test]
fn test_cli_v2_sample_conversion() {
    let output_file = "test_integration_output.xml";
    let _ = fs::remove_file(output_file); // Clean up if previous run failed
    
    // Run the compiled binary via cargo
    let output = Command::new("cargo")
        .args(&["run", "--", "--in", "res/v2_sample.xml", "--out", output_file])
        .output()
        .expect("Failed to execute cargo run");
        
    assert!(
        output.status.success(), 
        "CLI failed with error:\n{}", 
        String::from_utf8_lossy(&output.stderr)
    );
    
    // Validate output
    let generated = fs::read_to_string(output_file).expect("Failed to read test_integration_output.xml");
    
    assert!(
        generated.contains("http://crd.gov.pl/wzor/2025/12/19/14090/"), 
        "Missing updated V3 namespace!"
    );
    
    assert!(
        generated.contains("JPK_V7M (3)"), 
        "Missing updated KodFormularza!"
    );
    
    assert!(
        generated.contains("<DI>1</DI>"), 
        "Missing injected <DI> tags for WEW/RO rows!"
    );
    
    // Clean up
    let _ = fs::remove_file(output_file);
}
