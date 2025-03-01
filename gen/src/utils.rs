use anchor_lang::prelude::Pubkey;
use anchor_syn::idl::types::{Idl, IdlAccount};
use anyhow::Result;
use std::str::FromStr;

/// Calculate the IDL account address for a program
pub fn get_idl_address(program_id: &Pubkey) -> Pubkey {
    // This mimics the behavior in anchor_lang::idl::IdlAccount::address
    let seed = format!("anchor:idl:{}", program_id.to_string());
    Pubkey::find_program_address(&[seed.as_bytes()], program_id).0
}

/// Convert an anchor program's string representation to a Pubkey
pub fn parse_program_id(program_id_str: &str) -> Result<Pubkey> {
    Pubkey::from_str(program_id_str).map_err(|e| anyhow::anyhow!("Invalid program ID: {}", e))
}

/// Generate a simplified TypeScript interface from an IDL
pub fn generate_typescript(idl: &Idl) -> String {
    let mut ts = String::new();
    
    // Program interface
    ts.push_str(&format!("// TypeScript interface for {}\n", idl.name));
    ts.push_str("import * as anchor from '@coral-xyz/anchor';\n\n");
    
    // Type definitions
    if let Some(types) = &idl.types {
        for ty in types {
            ts.push_str(&format!("export interface {} {{\n", ty.name));
            
            // Add fields based on type
            match &ty.ty {
                anchor_syn::idl::types::IdlTypeDefinitionTy::Struct { fields } => {
                    for field in fields {
                        let ts_type = idl_type_to_ts(&field.ty);
                        ts.push_str(&format!("  {}: {};\n", field.name, ts_type));
                    }
                },
                anchor_syn::idl::types::IdlTypeDefinitionTy::Enum { variants } => {
                    // Enums are a bit more complex in TypeScript
                    // One approach is to model them as discriminated unions
                    for variant in variants {
                        if let Some(fields) = &variant.fields {
                            ts.push_str(&format!("  // {}:\n", variant.name));
                            for field in fields {
                                let ts_type = idl_type_to_ts(&field.ty);
                                ts.push_str(&format!("  // {}: {};\n", field.name, ts_type));
                            }
                        }
                    }
                },
                _ => {}
            }
            
            ts.push_str("}\n\n");
        }
    }
    
    // Program class
    ts.push_str(&format!("export class {}Program {{\n", idl.name));
    ts.push_str("  constructor(public program: anchor.Program) {}\n\n");
    
    // Add methods for each instruction
    for instruction in &idl.instructions {
        let args_str = instruction.args
            .iter()
            .map(|arg| format!("{}: {}", arg.name, idl_type_to_ts(&arg.ty)))
            .collect::<Vec<_>>()
            .join(", ");
        
        ts.push_str(&format!("  async {}({}) {{\n", instruction.name, args_str));
        ts.push_str("    // Implementation would call program methods\n");
        ts.push_str("  }\n\n");
    }
    
    ts.push_str("}\n");
    
    ts
}

/// Convert IDL type to TypeScript type
fn idl_type_to_ts(idl_type: &anchor_syn::idl::types::IdlType) -> String {
    match idl_type {
        anchor_syn::idl::types::IdlType::Bool => "boolean".to_string(),
        anchor_syn::idl::types::IdlType::U8 
        | anchor_syn::idl::types::IdlType::U16
        | anchor_syn::idl::types::IdlType::U32
        | anchor_syn::idl::types::IdlType::U64
        | anchor_syn::idl::types::IdlType::U128
        | anchor_syn::idl::types::IdlType::I8
        | anchor_syn::idl::types::IdlType::I16
        | anchor_syn::idl::types::IdlType::I32
        | anchor_syn::idl::types::IdlType::I64
        | anchor_syn::idl::types::IdlType::I128
        | anchor_syn::idl::types::IdlType::F32
        | anchor_syn::idl::types::IdlType::F64 => "number".to_string(),
        anchor_syn::idl::types::IdlType::PublicKey => "anchor.web3.PublicKey".to_string(),
        anchor_syn::idl::types::IdlType::String => "string".to_string(),
        anchor_syn::idl::types::IdlType::Array(item_type, size) => {
            format!("{}[]", idl_type_to_ts(item_type))
        },
        anchor_syn::idl::types::IdlType::Option(inner) => {
            format!("{} | null", idl_type_to_ts(inner))
        },
        anchor_syn::idl::types::IdlType::Defined(name) => name.clone(),
        // Handle other types as needed
        _ => "any".to_string(),
    }
}