use anchor_lang::prelude::Pubkey;
use anchor_syn::idl::types::Idl;
use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

mod compression;
mod utils;

/// Extract IDL from an Anchor program
pub fn extract_idl(program_path: &Path) -> Result<Idl> {
    if !program_path.exists() {
        return Err(anyhow!("Program path does not exist: {:?}", program_path));
    }
    
    // Determine if path is a directory or a file
    if program_path.is_dir() {
        // Look for lib.rs, main.rs, or the most likely entry point
        let potential_entry_points = [
            program_path.join("src/lib.rs"),
            program_path.join("lib.rs"),
            program_path.join("src/main.rs"),
            program_path.join("main.rs"),
        ];
        
        for entry_point in potential_entry_points {
            if entry_point.exists() {
                return extract_idl_from_file(&entry_point);
            }
        }
        
        // If no entry point found, search recursively for .rs files
        let rs_files = find_rs_files(program_path)?;
        if rs_files.is_empty() {
            return Err(anyhow!("No Rust files found in program directory"));
        }
        
        // Try to extract IDL from each file until successful
        for file in rs_files {
            match extract_idl_from_file(&file) {
                Ok(idl) => return Ok(idl),
                Err(_) => continue,
            }
        }
        
        Err(anyhow!("Could not extract IDL from any file in the directory"))
    } else {
        // Path is a file, try to extract directly
        extract_idl_from_file(program_path)
    }
}

/// Find all .rs files in a directory recursively
fn find_rs_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let mut sub_files = find_rs_files(&path)?;
                files.append(&mut sub_files);
            } else if path.extension().map_or(false, |ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    
    Ok(files)
}

/// Extract IDL from a specific file
fn extract_idl_from_file(file_path: &Path) -> Result<Idl> {
    println!("Attempting to extract IDL from: {:?}", file_path);
    
    let file_content = fs::read_to_string(file_path)?;
    
    // Use anchor_syn to extract the IDL
    let idl = match anchor_syn::idl::file::parse(&file_content) {
        Ok(idl) => idl,
        Err(e) => {
            println!("Failed to parse file: {}", e);
            return Err(anyhow!("Failed to parse file: {}", e));
        }
    };
    
    Ok(idl)
}

/// Write IDL to a file
pub fn write_idl(idl: &Idl, output_path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(idl)?;
    fs::write(output_path, json)?;
    Ok(())
}

/// Compress IDL using zlib
pub fn compress_idl(idl: &Idl) -> Result<Vec<u8>> {
    compression::compress_idl(idl)
}

/// Decompress IDL bytes
pub fn decompress_idl(data: &[u8]) -> Result<Idl> {
    compression::decompress_idl(data)
}

/// Calculate the IDL account address for a program
pub fn get_idl_address(program_id: &Pubkey) -> Pubkey {
    utils::get_idl_address(program_id)
}

/// Parse a program ID string into a Pubkey
pub fn parse_program_id(program_id_str: &str) -> Result<Pubkey> {
    utils::parse_program_id(program_id_str)
}

/// Generate TypeScript interfaces from an IDL
pub fn generate_typescript(idl: &Idl) -> String {
    utils::generate_typescript(idl)
}