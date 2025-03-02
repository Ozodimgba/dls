use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use anyhow::{Context, Result, anyhow};
use clap::Parser;
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "anchor-idl-extractor",
    author = "FX",
    version = "0.1.7",
    about = "Extract IDL from template and update with program ID"
)]
struct Cli {
    /// Path to the program source
    #[arg(short, long)]
    program_path: PathBuf,
    
    /// Path to the template IDL JSON file
    #[arg(short, long)]
    template: PathBuf,

    /// Output file path (defaults to target/idl/program_name.json)
    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    if cli.verbose {
        println!("Anchor IDL Extractor");
    }

    // 1. Read the template IDL file
    let template_content = fs::read_to_string(&cli.template)
        .context("Failed to read template IDL file")?;
    
    let mut idl: Value = serde_json::from_str(&template_content)
        .context("Failed to parse template IDL as JSON")?;
    
    // Get the program name from the IDL
    let program_name = idl["name"].as_str()
        .ok_or_else(|| anyhow!("Program name not found in template IDL"))?
        .to_string();
    
    if cli.verbose {
        println!("Program name from IDL: {}", program_name);
    }
    
    // 2. Extract program ID from source
    let lib_rs_path = cli.program_path.join("src/lib.rs");
    
    if cli.verbose {
        println!("Reading program ID from {}", lib_rs_path.display());
    }

    let source_code = fs::read_to_string(&lib_rs_path)
        .context("Failed to read src/lib.rs")?;

    let program_id = extract_program_id(&source_code)
        .context("Failed to extract program ID")?;
    
    if cli.verbose {
        println!("Program ID: {}", program_id);
    }
    
    // 3. Update the program ID in the IDL
    idl["address"] = Value::String(program_id.clone());
    
    // 4. Write the updated IDL to the output file
    let output_path = match cli.output {
        Some(path) => path,
        None => {
            let target_dir = cli.program_path.join("target/idl");
            fs::create_dir_all(&target_dir)
                .context("Failed to create target/idl directory")?;
            target_dir.join(format!("{}.json", program_name))
        }
    };
    
    let idl_json = serde_json::to_string_pretty(&idl)
        .context("Failed to serialize IDL to JSON")?;
    
    write_idl_to_file(&output_path, idl_json)?;

    println!("IDL extracted and updated successfully at {}", output_path.display());
    println!("Program ID: {}", program_id);
    
    Ok(())
}

fn extract_program_id(source_code: &str) -> Result<String> {
    // Find the declare_id! macro call
    let declare_id_line = source_code.lines()
        .find(|line| line.trim().starts_with("declare_id!"))
        .ok_or_else(|| anyhow!("Could not find declare_id! in source code"))?;
    
    // Extract the program ID from the macro
    let start = declare_id_line.find('"')
        .ok_or_else(|| anyhow!("Invalid declare_id! format"))? + 1;
    let end = declare_id_line[start..].find('"')
        .ok_or_else(|| anyhow!("Invalid declare_id! format"))? + start;
    
    Ok(declare_id_line[start..end].to_string())
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