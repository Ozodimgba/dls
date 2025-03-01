use anchor_idl_gen::{extract_idl, write_idl, parse_program_id, get_idl_address, generate_typescript};
use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about = "Anchor IDL Generator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract IDL from an Anchor program
    Extract {
        /// Path to the program source directory or entry file
        #[arg(short, long)]
        program: PathBuf,
        
        /// Output path for the IDL JSON file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Get IDL account address for a program
    IdlAddress {
        /// Program ID (base58 encoded)
        #[arg(short, long)]
        program_id: String,
    },
    
    /// Generate TypeScript interfaces from an IDL
    GenerateTs {
        /// Path to the IDL JSON file
        #[arg(short, long)]
        idl: PathBuf,
        
        /// Output path for the TypeScript file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Extract { program, output } => {
            println!("Extracting IDL from program at {:?}", program);
            
            let idl = extract_idl(&program)?;
            println!("Successfully extracted IDL for program: {}", idl.name);
            
            if let Some(output_path) = output {
                write_idl(&idl, &output_path)?;
                println!("IDL written to: {:?}", output_path);
            } else {
                let json = serde_json::to_string_pretty(&idl)?;
                println!("{}", json);
            }
        },
        
        Commands::IdlAddress { program_id } => {
            let pubkey = parse_program_id(&program_id)?;
            let idl_address = get_idl_address(&pubkey);
            println!("Program ID: {}", program_id);
            println!("IDL Account Address: {}", idl_address);
        },
        
        Commands::GenerateTs { idl, output } => {
            if !idl.exists() {
                return Err(anyhow!("IDL file does not exist: {:?}", idl));
            }
            
            let idl_content = fs::read_to_string(&idl)?;
            let parsed_idl: anchor_syn::idl::types::Idl = serde_json::from_str(&idl_content)?;
            
            let typescript = generate_typescript(&parsed_idl);
            
            if let Some(output_path) = output {
                fs::write(&output_path, typescript)?;
                println!("TypeScript interfaces written to: {:?}", output_path);
            } else {
                println!("{}", typescript);
            }
        }
    }
    
    Ok(())
}