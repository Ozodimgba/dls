use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use anyhow::{Context, Result, anyhow};
use clap::Parser;

// Directly use the private modules from anchor-syn that we need
use anchor_syn::{
    Program,
    parser::program as program_parser,
    idl,
};


#[derive(Parser)]
#[command(
    name = "dls",
    author = "FX",
    version = "0.1.4",
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

    let program_id = extract_program_id(&source_code)
        .context("Failed to extract program ID")?;
    
    if cli.verbose {
        println!("Program ID: {}", program_id);
    }

    let file = syn::parse_file(&source_code)
        .context("Failed to parse Rust source code")?;
    
    // Find the module with #[program] attribute
    let program_mod = file.items.iter()
        .find_map(|item| {
            if let syn::Item::Mod(item_mod) = item {
                // Check if the module has the #[program] attribute
                if item_mod.attrs.iter().any(|attr| {
                    attr.path.is_ident("program")
                }) {
                    Some(item_mod.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow!("Could not find a module with #[program] attribute in file"))?;
    
    if cli.verbose {
        println!("Found program module: {}", program_mod.ident);
    }

    let program = program_parser::parse(program_mod)
        .context("Failed to parse Anchor program structure")?;

    if cli.verbose {
        println!("Generating IDL...");
    }
    
    // Use Anchor's IDL generation functionality
    // We need to create a full IDL structure manually using the parts from anchor-syn
    let mut idl = serde_json::json!({
        "version": "0.1.0",
        "name": program.name.to_string(),
        "instructions": [],
        "accounts": [],
        "events": [],
        "errors": [],
        "address": program_id
    });

    // Generate instructions
    let instructions: Vec<serde_json::Value> = program.ixs.iter().map(|ix| {
        let name = ix.ident.to_string();
        let accounts = ix.anchor_ident.to_string();
        
        // Extract argument types
        let args: Vec<serde_json::Value> = ix.args.iter().map(|arg| {
            let arg_name = arg.name.to_string();
            let arg_type = extract_type_string(&arg.raw_arg.ty);
            
            serde_json::json!({
                "name": arg_name,
                "type": arg_type
            })
        }).collect();
        
        serde_json::json!({
            "name": name,
            "accounts": accounts,
            "args": args
        })
    }).collect();
    
    idl["instructions"] = serde_json::Value::Array(instructions);
    
    let idl_json = serde_json::to_string_pretty(&idl)
        .context("Failed to serialize IDL to JSON")?;
    
    let output_path = match cli.output {
        Some(path) => path,
        None => {
            let target_dir = project_root.join("target/idl");
            fs::create_dir_all(&target_dir)
                .context("Failed to create target/idl directory")?;
            target_dir.join(format!("{}.json", program.name))
        }
    };
    
    write_idl_to_file(&output_path, idl_json)?;

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

// Helper function to extract type information from Rust types
fn extract_type_string(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => {
            let path_string = type_path.path.segments.iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            // Handle common primitive types
            match path_string.as_str() {
                "u8" => "u8".to_string(),
                "u16" => "u16".to_string(),
                "u32" => "u32".to_string(),
                "u64" => "u64".to_string(),
                "u128" => "u128".to_string(),
                "i8" => "i8".to_string(),
                "i16" => "i16".to_string(),
                "i32" => "i32".to_string(),
                "i64" => "i64".to_string(),
                "i128" => "i128".to_string(),
                "f32" => "f32".to_string(),
                "f64" => "f64".to_string(),
                "bool" => "bool".to_string(),
                "String" => "string".to_string(),
                "str" => "string".to_string(),
                "Pubkey" => "publicKey".to_string(),
                _ => {
                    // Check for generic types
                    if let Some(last_segment) = type_path.path.segments.last() {
                        if last_segment.ident == "Option" {
                            // Handle Option<T>
                            if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                                    return format!("option<{}>", extract_type_string(inner_type));
                                }
                            }
                        } else if last_segment.ident == "Vec" {
                            // Handle Vec<T>
                            if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                                    return format!("vec<{}>", extract_type_string(inner_type));
                                }
                            }
                        }
                    }
                    
                    // For other types, just use the last segment as the type name
                    if let Some(last_segment) = type_path.path.segments.last() {
                        last_segment.ident.to_string()
                    } else {
                        "unknown".to_string()
                    }
                }
            }
        },
        syn::Type::Reference(type_ref) => {
            // For references, extract the inner type
            extract_type_string(&type_ref.elem)
        },
        syn::Type::Array(type_array) => {
            // Handle arrays like [u8; 32]
            let elem_type = extract_type_string(&type_array.elem);
            
            match &type_array.len {
                syn::Expr::Lit(lit) => {
                    if let syn::Lit::Int(int) = &lit.lit {
                        let size = int.base10_parse::<usize>().unwrap_or(0);
                        format!("array<{}, {}>", elem_type, size)
                    } else {
                        format!("array<{}>", elem_type)
                    }
                },
                _ => format!("array<{}>", elem_type)
            }
        },
        _ => "unknown".to_string()
    }
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