use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use anyhow::{Context, Result, anyhow};
use clap::Parser;

// Directly use the private modules from anchor-syn that we need
use anchor_syn::{
    Program,
    parser::program as program_parser,
};


#[derive(Parser)]
#[command(
    name = "dls",
    author = "FX",
    version = "0.1.1",
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
        println!("IDL Generator using anchor-syn");
    }

    let project_root = match cli.program_path {
        Some(path) => path,
        None => find_project_root().context("Failed to find project root")?,
    };
    
    if cli.verbose {
        println!("Project root: {}", project_root.display());
    }

    let lib_rs_path = project_root.join("src/lib.rs");
    
    if cli.verbose {
        println!("Parsing {}", lib_rs_path.display());
    }

    let source_code = fs::read_to_string(&lib_rs_path)
        .context("Failed to read src/lib.rs")?;

    // Step 4: Extract the program ID
    let program_id = extract_program_id(&source_code)
        .context("Failed to extract program ID")?;
    
    if cli.verbose {
        println!("Program ID: {}", program_id);
    }

    let file = syn::parse_file(&source_code)
        .context("Failed to parse Rust source code")?;
    
    let program_mod = file.items.iter()
        .find_map(|item| {
            if let syn::Item::Mod(item_mod) = item {
                Some(item_mod.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow!("Could not find program module in file"))?;
    
    let program = program_parser::parse(program_mod)
        .context("Failed to parse Anchor program structure")?;

    if cli.verbose {
        println!("Generating IDL...");
    }
    let idl = generate_idl(&program, &program_id)?;
    
    let output_path = match cli.output {
        Some(path) => path,
        None => {
            let target_dir = project_root.join("target/idl");
            fs::create_dir_all(&target_dir)
                .context("Failed to create target/idl directory")?;
            target_dir.join(format!("{}.json", program.name))
        }
    };
    
    write_idl_to_file(&output_path, idl)?;

    println!("IDL generated successfully at {}", output_path.display());
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

fn generate_idl(program: &Program, program_id: &str) -> Result<String> {
    // Manual IDL generation based on the program structure
    let mut idl = serde_json::json!({
        "version": "0.1.0",
        "name": program.name.to_string(),
        "instructions": [],
        "accounts": [],
        "events": [],
        "errors": [],
        "address": program_id
    });
    
    // Add instructions
    let instructions = program.ixs.iter().map(|ix| {
        serde_json::json!({
            "name": ix.ident.to_string(),
            "accounts": ix.anchor_ident.to_string(),
            "args": ix.args.iter().map(|arg| {
                serde_json::json!({
                    "name": arg.name.to_string(),
                    "type": "unknown" // We would need more parsing to get the real type
                })
            }).collect::<Vec<_>>()
        })
    }).collect::<Vec<_>>();
    
    idl["instructions"] = serde_json::Value::Array(instructions);
    
    let idl_json = serde_json::to_string_pretty(&idl)
        .context("Failed to serialize IDL to JSON")?;
    
    Ok(idl_json)
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