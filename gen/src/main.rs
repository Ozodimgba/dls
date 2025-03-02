use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use std::process::{Command, Stdio};
use anyhow::{Context, Result, anyhow};
use clap::Parser;

// We'll use the same approach that Anchor uses internally
// to generate IDL files via the compilation process

#[derive(Parser)]
#[command(
    name = "dls",
    author = "FX",
    version = "0.1.5",
    about = "Generate IDL for Anchor programs without workspace"
)]
struct Cli {
    #[arg(short, long)]
    program_path: Option<PathBuf>,

    /// Output file path (defaults to target/idl/program_name.json)
    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    if cli.verbose {
        println!("IDL Generator using anchor build process");
    }

    let project_root = match cli.program_path {
        Some(path) => path,
        None => find_project_root().context("Failed to find project root")?,
    };
    
    if cli.verbose {
        println!("Project root: {}", project_root.display());
    }

    // Extract program name from Cargo.toml
    let cargo_toml_path = project_root.join("Cargo.toml");
    let cargo_toml = fs::read_to_string(&cargo_toml_path)
        .context("Failed to read Cargo.toml")?;
    let program_name = extract_program_name(&cargo_toml)
        .context("Failed to extract program name from Cargo.toml")?;
    
    if cli.verbose {
        println!("Program name: {}", program_name);
    }

    // Extract program ID from lib.rs
    let lib_rs_path = project_root.join("src/lib.rs");
    let source_code = fs::read_to_string(&lib_rs_path)
        .context("Failed to read src/lib.rs")?;
    let program_id = extract_program_id(&source_code)
        .context("Failed to extract program ID")?;
    
    if cli.verbose {
        println!("Program ID: {}", program_id);
    }

    // Setup temporary Cargo.toml with idl-build feature
    prepare_for_idl_build(&project_root)?;

    // Generate the IDL using Anchor's build process
    if cli.verbose {
        println!("Generating IDL...");
    }
    
    generate_idl(&project_root, cli.verbose)?;
    
    // Get the IDL file path
    let idl_path = project_root.join("target/idl").join(format!("{}.json", program_name));
    
    // If the IDL file doesn't exist, fallback to using output from stderr
    if !idl_path.exists() {
        return Err(anyhow!("IDL generation failed. IDL file not found at {}", idl_path.display()));
    }
    
    // Read the IDL file
    let idl_json = fs::read_to_string(&idl_path)
        .context("Failed to read generated IDL file")?;
    
    // Update the program ID in the IDL
    let mut idl: serde_json::Value = serde_json::from_str(&idl_json)
        .context("Failed to parse IDL JSON")?;
    
    idl["address"] = serde_json::Value::String(program_id);
    
    // Write the IDL to the output file
    let output_path = match cli.output {
        Some(path) => path,
        None => idl_path,
    };
    
    let updated_idl_json = serde_json::to_string_pretty(&idl)
        .context("Failed to serialize IDL to JSON")?;
    
    write_idl_to_file(&output_path, updated_idl_json)?;

    if cli.verbose {
        println!("IDL generated successfully at {}", output_path.display());
    } else {
        println!("IDL generated successfully");
    }
    
    Ok(())
}

fn find_project_root() -> Result<PathBuf> {
    let mut current_dir = std::env::current_dir()?;
    
    loop {
        let cargo_toml = current_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            return Ok(current_dir);
        }
        
        if !current_dir.pop() {
            return Err(anyhow!("Could not find Cargo.toml in any parent directory"));
        }
    }
}

fn extract_program_name(cargo_toml: &str) -> Result<String> {
    let regex = regex::Regex::new(r#"name\s*=\s*"([^"]+)""#)?;
    match regex.captures(cargo_toml) {
        Some(caps) => Ok(caps[1].to_string()),
        None => Err(anyhow!("Could not find package name in Cargo.toml")),
    }
}

fn extract_program_id(source_code: &str) -> Result<String> {
    let declare_id_line = source_code.lines()
        .find(|line| line.trim().starts_with("declare_id!"))
        .ok_or_else(|| anyhow!("Could not find declare_id! in source code"))?;
    
    let start = declare_id_line.find('"')
        .ok_or_else(|| anyhow!("Invalid declare_id! format"))? + 1;
    let end = declare_id_line[start..].find('"')
        .ok_or_else(|| anyhow!("Invalid declare_id! format"))? + start;
    
    Ok(declare_id_line[start..end].to_string())
}

fn prepare_for_idl_build(project_root: &Path) -> Result<()> {
    // Ensure the .cargo directory exists
    let cargo_dir = project_root.join(".cargo");
    fs::create_dir_all(&cargo_dir)?;
    
    // Create a config.toml file with nightly toolchain
    let config_content = r#"
[build]
rustflags = ["--cfg", "feature=\"idl-build\""]

[unstable]
build-std = ["std", "panic_abort"]
"#;
    
    let config_path = cargo_dir.join("config.toml");
    fs::write(&config_path, config_content)?;
    
    Ok(())
}

fn generate_idl(project_root: &Path, verbose: bool) -> Result<()> {
    // Set the environment variables needed for IDL generation
    let env_vars = [
        ("RUSTFLAGS", "--cfg procmacro2_semver_exempt"),
        ("CARGO_ENCODED_RUSTFLAGS", "--cfg%20procmacro2_semver_exempt"),
        ("ANCHOR_IDL_BUILD_RESOLUTION", "TRUE"),
        ("ANCHOR_IDL_BUILD_SKIP_LINT", "TRUE"),
        ("RUSTUP_TOOLCHAIN", "nightly"),
    ];
    
    // Create the command
    let mut command = Command::new("cargo");
    
    // Set working directory
    command.current_dir(project_root);
    
    // Set the command
    command.args([
        "test", 
        "__anchor_private_print_idl", 
        "--features", 
        "idl-build", 
        "--", 
        "--show-output"
    ]);
    
    // Add environment variables
    for (key, value) in env_vars.iter() {
        command.env(key, value);
    }
    
    // Stdout/stderr handling
    if verbose {
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());
    } else {
        command.stdout(Stdio::null());
        command.stderr(Stdio::piped());
    }
    
    // Run the command
    let output = command.output()
        .context("Failed to execute cargo test for IDL generation")?;
    
    // Check if the command was successful
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("IDL generation failed: {}", stderr));
    }
    
    Ok(())
}

fn write_idl_to_file(output_path: &Path, idl_json: String) -> Result<()> {
    // Create parent directories if needed
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create output directory")?;
    }
    
    // Write IDL to file
    let mut file = fs::File::create(output_path)
        .context("Failed to create IDL file")?;
    
    file.write_all(idl_json.as_bytes())
        .context("Failed to write IDL JSON to file")?;
    
    Ok(())
}