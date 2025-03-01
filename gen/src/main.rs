use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, exit};

use clap::{Arg, Command};

fn main() {
    // Make sure we have the build feature enabled
    let matches = Command::new("anchor-idl-gen")
        .version("0.1.0")
        .about("Generate IDL for Anchor programs without initializing a workspace")
        .arg(
            Arg::new("program-path")
                .short('p')
                .long("program-path")
                .help("Path to the Anchor program")
                .required(false),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output file for the IDL JSON")
                .required(false),
        )
        .arg(
            Arg::new("no-docs")
                .long("no-docs")
                .help("Skip generating docs in the IDL")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Get program path
    let program_path = match matches.get_one::<String>("program-path") {
        Some(path) => PathBuf::from(path),
        None => env::current_dir().expect("Failed to get current directory"),
    };

    // Validate program path
    if !program_path.exists() {
        eprintln!("Error: Program path does not exist: {:?}", program_path);
        exit(1);
    }

    // Check if nightly toolchain is installed
    println!("Checking if nightly toolchain is installed...");
    let nightly_check = ProcessCommand::new("rustup")
        .args(["toolchain", "list"])
        .output();
        
    if let Ok(output) = nightly_check {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if !output_str.contains("nightly") {
            println!("Nightly toolchain not found, installing...");
            let _ = ProcessCommand::new("rustup")
                .args(["toolchain", "install", "nightly"])
                .status();
        }
    }

    println!("Building IDL for program at {:?}", program_path);
    println!("This may take a moment as we need to compile your program...");
    
    // Use the build_idl function directly to avoid module path issues
    #[allow(deprecated)]
    let idl = match anchor_lang_idl::build::build_idl(
        &program_path,
        true, // resolution
        false, // skip_lint
        matches.get_flag("no-docs"),
    ) {
        Ok(idl) => idl,
        Err(err) => {
            eprintln!("Error building IDL: {}", err);
            eprintln!("Make sure your program builds correctly with 'cargo build-sbf'");
            eprintln!("Also ensure you have the nightly Rust toolchain installed");
            exit(1);
        }
    };

    // Serialize to JSON
    let json = match serde_json::to_string_pretty(&idl) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("Error serializing IDL to JSON: {}", err);
            exit(1);
        }
    };

    // Output file path
    let output_file = match matches.get_one::<String>("output") {
        Some(path) => PathBuf::from(path),
        None => {
            let program_name = &idl.metadata.name;
            program_path.join(format!("{}.json", program_name))
        }
    };

    // Create parent directory if it doesn't exist
    if let Some(parent) = output_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).expect("Failed to create output directory");
        }
    }

    // Write to file
    match fs::write(&output_file, json) {
        Ok(_) => {
            println!("✅ IDL successfully generated!");
            println!("IDL written to {:?}", output_file);
            println!("Program name: {}", idl.metadata.name);
            println!("Program version: {}", idl.metadata.version);
            println!("Instructions: {}", idl.instructions.len());
            println!("Accounts: {}", idl.accounts.len());
            println!("Types: {}", idl.types.len());
        },
        Err(err) => {
            eprintln!("Error writing IDL to file: {}", err);
            exit(1);
        }
    }
}