use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

/// CLI tool for generating Anchor IDLs without the full Anchor CLI
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,

        #[arg(short, long)]
        output: Option<PathBuf>,

        #[arg(long)]
        skip_lint: bool,

        #[arg(long)]
        no_docs: bool,

        #[arg(long)]
        no_resolution: bool,
    },

    // Convert an IDL from a legacy format to the current format
    Convert {
        #[arg(short, long)]
        input: PathBuf,

        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    

    Validate {
        #[arg(short, long)]
        input: PathBuf,
    },
    
    Instructions {
        #[arg(short, long)]
        input: PathBuf,
        
        #[arg(long)]
        names_only: bool,
    },
}

fn display_instructions(path: &PathBuf, names_only: bool) -> Result<()> {
    debug!("Extracting instructions from IDL at: {:?}", path);
    
    // Read the IDL file
    let idl_bytes = fs::read(path)
        .with_context(|| format!("Failed to read IDL file at {:?}", path))?;
    
    // Parse the IDL
    let idl = anchor_lang_idl::convert::convert_idl(&idl_bytes)
        .context("Failed to parse IDL")?;
    
    println!("\nProgram: {} (v{})", idl.metadata.name, idl.metadata.version);
    println!("Address: {}", idl.address);
    println!("\nInstructions ({}):", idl.instructions.len());
    
    for (idx, instruction) in idl.instructions.iter().enumerate() {
        println!("\n{}. {}", idx + 1, instruction.name);
        
        if !names_only {
            // Show documentation if available
            if !instruction.docs.is_empty() {
                println!("   Description:");
                for doc in &instruction.docs {
                    println!("     {}", doc);
                }
            }

            if !instruction.args.is_empty() {
                println!("   Arguments:");
                for arg in &instruction.args {
                    println!("     {} ({})", arg.name, format_type(&arg.ty));
                    if !arg.docs.is_empty() {
                        println!("       {}", arg.docs.join(" "));
                    }
                }
            } else {
                println!("   Arguments: None");
            }
            
            // Show accounts
            println!("   Accounts:");
            if instruction.accounts.is_empty() {
                println!("     None");
            } else {
                display_accounts(&instruction.accounts, 1);
            }
            
            if let Some(returns) = &instruction.returns {
                println!("   Returns: {}", format_type(returns));
            }
        }
    }
    
    Ok(())
}

// Recursively display accounts with proper indentation
fn display_accounts(accounts: &[anchor_lang_idl::types::IdlInstructionAccountItem], depth: usize) {
    use anchor_lang_idl::types::IdlInstructionAccountItem;
    
    let indent = "  ".repeat(depth + 2);
    
    for account in accounts {
        match account {
            IdlInstructionAccountItem::Single(acc) => {
                let mut attrs = Vec::new();
                if acc.writable {
                    attrs.push("writable");
                }
                if acc.signer {
                    attrs.push("signer");
                }
                if acc.optional {
                    attrs.push("optional");
                }
                
                let attr_str = if attrs.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", attrs.join(", "))
                };
                
                println!("{}{}{}", indent, acc.name, attr_str);
                
                if let Some(pda) = &acc.pda {
                    println!("{}  PDA with {} seeds", indent, pda.seeds.len());
                }
            },
            IdlInstructionAccountItem::Composite(composite) => {
                println!("{}{}:", indent, composite.name);
                display_accounts(&composite.accounts, depth + 1);
            }
        }
    }
}

/// Format an IDL type for display
fn format_type(ty: &anchor_lang_idl::types::IdlType) -> String {
    use anchor_lang_idl::types::{IdlArrayLen, IdlGenericArg, IdlType};
    
    match ty {
        IdlType::Bool => "bool".into(),
        IdlType::U8 => "u8".into(),
        IdlType::I8 => "i8".into(),
        IdlType::U16 => "u16".into(),
        IdlType::I16 => "i16".into(),
        IdlType::U32 => "u32".into(),
        IdlType::I32 => "i32".into(),
        IdlType::F32 => "f32".into(),
        IdlType::U64 => "u64".into(),
        IdlType::I64 => "i64".into(),
        IdlType::F64 => "f64".into(),
        IdlType::U128 => "u128".into(),
        IdlType::I128 => "i128".into(),
        IdlType::U256 => "u256".into(),
        IdlType::I256 => "i256".into(),
        IdlType::Bytes => "bytes".into(),
        IdlType::String => "string".into(),
        IdlType::Pubkey => "pubkey".into(),
        IdlType::Option(inner) => format!("Option<{}>", format_type(inner)),
        IdlType::Vec(inner) => format!("Vec<{}>", format_type(inner)),
        IdlType::Array(inner, len) => match len {
            IdlArrayLen::Value(size) => format!("[{}; {}]", format_type(inner), size),
            IdlArrayLen::Generic(name) => format!("[{}; {}]", format_type(inner), name),
        },
        IdlType::Defined { name, generics } => {
            if generics.is_empty() {
                name.clone()
            } else {
                let generic_strs: Vec<String> = generics
                    .iter()
                    .map(|g| match g {
                        IdlGenericArg::Type { ty } => format_type(ty),
                        IdlGenericArg::Const { value } => value.clone(),
                    })
                    .collect();
                format!("{}<{}>", name, generic_strs.join(", "))
            }
        }
        IdlType::Generic(name) => name.clone(),
        // wildcard pattern for any new types added in the future
        _ => format!("<unknown type>"),
    }
}

// Validates an IDL file against specification
fn validate_idl(path: &PathBuf) -> Result<()> {
    debug!("Validating IDL at: {:?}", path);
    
    // Read the IDL file
    let idl_bytes = fs::read(path)
        .with_context(|| format!("Failed to read IDL file at {:?}", path))?;
    
    // Try to parse it as the current IDL format
    let idl_result = anchor_lang_idl::convert::convert_idl(&idl_bytes);
    
    match idl_result {
        Ok(idl) => {
            // validation
            if idl.address.is_empty() {
                return Err(anyhow::anyhow!("IDL is missing program address"));
            }
            
            if idl.metadata.name.is_empty() {
                return Err(anyhow::anyhow!("IDL is missing program name"));
            }
            
            if idl.metadata.version.is_empty() {
                return Err(anyhow::anyhow!("IDL is missing version"));
            }
            
            // Check for empty discriminators
            for account in &idl.accounts {
                if account.discriminator.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Account '{}' has an empty discriminator", 
                        account.name
                    ));
                }
            }
            
            for instruction in &idl.instructions {
                if instruction.discriminator.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Instruction '{}' has an empty discriminator", 
                        instruction.name
                    ));
                }
            }
            
            for event in &idl.events {
                if event.discriminator.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Event '{}' has an empty discriminator", 
                        event.name
                    ));
                }
            }
            
            info!("IDL validation successful!");
            info!("Program: {}", idl.metadata.name);
            info!("Version: {}", idl.metadata.version);
            info!("Accounts: {}", idl.accounts.len());
            info!("Instructions: {}", idl.instructions.len());
            info!("Types: {}", idl.types.len());
            
            Ok(())
        },
        Err(e) => {
            Err(anyhow::anyhow!("IDL validation failed: {}", e))
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // logging based n verbosity flag
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("anchor_idl_cli={},anchor_lang_idl={}", log_level, log_level))
        .init();

    match &cli.command {
        Commands::Build {
            path,
            output,
            skip_lint,
            no_docs,
            no_resolution,
        } => {
            debug!("Building IDL for program at: {:?}", path);
            
            // Directly call the build_idl function - using #[allow(deprecated)] to avoid warnings
            #[allow(deprecated)]
            let idl = anchor_lang_idl::build::build_idl(
                path,
                !no_resolution,
                *skip_lint,
                *no_docs
            ).context("Failed to build IDL")?;
            
            // Serialize the IDL to JSON with pretty printing
            let idl_json = anchor_lang_idl::serde_json::to_string_pretty(&idl)
                .context("Failed to serialize IDL to JSON")?;
            
            // Determine output path
            let output_path = match output {
                Some(path) => path.clone(),
                None => {
                    let program_name = &idl.metadata.name;
                    PathBuf::from(format!("{}.json", program_name))
                }
            };
            
            // Write the IDL to the output file
            fs::write(&output_path, idl_json)
                .with_context(|| format!("Failed to write IDL to {:?}", output_path))?;
            
            info!("Successfully built IDL and saved to {:?}", output_path);
        }
        
        Commands::Convert { input, output } => {
            debug!("Converting IDL from: {:?}", input);
            
            // Read the input IDL file
            let idl_bytes = fs::read(input)
                .with_context(|| format!("Failed to read IDL file at {:?}", input))?;
            
            // Convert the IDL
            let converted_idl = anchor_lang_idl::convert::convert_idl(&idl_bytes)
                .context("Failed to convert IDL")?;
            
            // Serialize the converted IDL to JSON with pretty printing
            let idl_json = anchor_lang_idl::serde_json::to_string_pretty(&converted_idl)
                .context("Failed to serialize converted IDL to JSON")?;
            
            // Determine output path
            let output_path = match output {
                Some(path) => path.clone(),
                None => {
                    let input_stem = input.file_stem().unwrap_or_default();
                    let mut output_path = input.with_file_name(input_stem);
                    output_path.set_extension("converted.json");
                    output_path
                }
            };
            
            // Write the converted IDL to the output file
            fs::write(&output_path, idl_json)
                .with_context(|| format!("Failed to write converted IDL to {:?}", output_path))?;
            
            info!("Successfully converted IDL and saved to {:?}", output_path);
        }
        
        Commands::Validate { input } => {
            validate_idl(input)?;
        }
        
        Commands::Instructions { input, names_only } => {
            display_instructions(input, *names_only)?;
        }
    }

    Ok(())
}