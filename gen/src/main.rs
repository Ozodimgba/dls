use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use anyhow::{Context, Result, anyhow};
use clap::Parser;
use std::fmt::Write as FmtWrite;
use std::collections::HashSet;
use syn::__private::ToTokens; // Add this import for the ToTokens trait

// Directly use the private modules from anchor-syn that we need
use anchor_syn::{
    Program,
    parser::program as program_parser,
};


#[derive(Parser)]
#[command(
    name = "dls",
    author = "FX",
    version = "0.1.3",
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

// Enhanced IDL generator function
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
                    "type": extract_type_from_arg(&arg.raw_arg.ty)
                })
            }).collect::<Vec<_>>()
        })
    }).collect::<Vec<_>>();
    
    idl["instructions"] = serde_json::Value::Array(instructions);
    
    // Try to extract account structures as well
    let mut accounts_set = HashSet::new();
    for ix in &program.ixs {
        accounts_set.insert(ix.anchor_ident.to_string());
    }
    
    // If you have account struct definitions available, you could add them here
    
    let idl_json = serde_json::to_string_pretty(&idl)
        .context("Failed to serialize IDL to JSON")?;
    
    Ok(idl_json)
}

// Helper function to extract type information from a Rust type
fn extract_type_from_arg(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => {
            let path_string = path_to_string(&type_path.path);
            
            // Handle common types
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
                "String" | "str" => "string".to_string(),
                "Pubkey" => "publicKey".to_string(),
                _ => {
                    // Check if it's an Option<T>
                    if path_string.starts_with("Option<") {
                        if let Some(inner_type) = extract_inner_type(type_path) {
                            return format!("option<{}>", inner_type);
                        }
                    }
                    
                    // Check if it's a Vec<T>
                    if path_string.starts_with("Vec<") {
                        if let Some(inner_type) = extract_inner_type(type_path) {
                            return format!("vec<{}>", inner_type);
                        }
                    }
                    
                    // For other types, we'll just use their path as a string
                    path_string
                }
            }
        },
        syn::Type::Reference(type_ref) => {
            // For references, extract the inner type
            extract_type_from_arg(&type_ref.elem)
        },
        syn::Type::Array(type_array) => {
            // For arrays like [u8; 32], represent as array<type>
            let elem_type = extract_type_from_arg(&type_array.elem);
            let size = match &type_array.len {
                syn::Expr::Lit(lit) => {
                    if let syn::Lit::Int(int) = &lit.lit {
                        int.base10_parse::<usize>().unwrap_or(0).to_string()
                    } else {
                        "unknown_size".to_string()
                    }
                },
                _ => "unknown_size".to_string()
            };
            format!("array<{}, {}>", elem_type, size)
        },
        syn::Type::Tuple(type_tuple) => {
            let mut tuple_types = Vec::new();
            for elem in &type_tuple.elems {
                tuple_types.push(extract_type_from_arg(elem));
            }
            format!("tuple<{}>", tuple_types.join(", "))
        },
        _ => "unknown".to_string(), // Fallback for other types
    }
}

// Helper to extract the inner type from a generic type like Option<T> or Vec<T>
fn extract_inner_type(type_path: &syn::TypePath) -> Option<String> {
    if type_path.path.segments.is_empty() {
        return None;
    }
    
    let last_segment = type_path.path.segments.last().unwrap();
    
    if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
        for arg in &args.args {
            if let syn::GenericArgument::Type(inner_type) = arg {
                return Some(extract_type_from_arg(inner_type));
            }
        }
    }
    
    None
}

// Helper to convert a path to a string representation
fn path_to_string(path: &syn::Path) -> String {
    let mut result = String::new();
    
    for (i, segment) in path.segments.iter().enumerate() {
        if i > 0 {
            result.push_str("::");
        }
        
        result.push_str(&segment.ident.to_string());
        
        // Handle generic arguments
        match &segment.arguments {
            syn::PathArguments::None => {},
            syn::PathArguments::AngleBracketed(args) => {
                result.push('<');
                for (j, arg) in args.args.iter().enumerate() {
                    if j > 0 {
                        result.push_str(", ");
                    }
                    match arg {
                        syn::GenericArgument::Type(ty) => {
                            let _ = write!(result, "{}", extract_type_from_arg(ty));
                        },
                        _ => {
                            let _ = write!(result, "{}", arg.into_token_stream());
                        }
                    }
                }
                result.push('>');
            },
            syn::PathArguments::Parenthesized(args) => {
                result.push('(');
                for (j, input) in args.inputs.iter().enumerate() {
                    if j > 0 {
                        result.push_str(", ");
                    }
                    let _ = write!(result, "{}", extract_type_from_arg(input));
                }
                result.push(')');
                
                // Handle output type if present
                if let syn::ReturnType::Type(_, ty) = &args.output {
                    result.push_str(" -> ");
                    let _ = write!(result, "{}", extract_type_from_arg(ty));
                }
            }
        }
    }
    
    result
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