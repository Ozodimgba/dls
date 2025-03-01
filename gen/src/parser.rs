// src/parser.rs
use anyhow::{Result, anyhow};
use anchor_syn::idl::types::{IdlAccountItem, IdlField, IdlType, IdlEnumVariant};
use std::path::{Path, PathBuf};
use std::fs;
use syn::{parse_file, Item, ItemStruct, ItemEnum, ItemFn, Attribute};
use syn::punctuated::Punctuated;
use syn::token::Comma;

// Structures to represent parsed program elements
pub struct ParsedProgram {
    pub name: String,
    pub docs: Option<Vec<String>>,
    pub instructions: Vec<ParsedInstruction>,
    pub accounts: Vec<ParsedAccount>,
    pub types: Vec<ParsedType>,
    pub events: Vec<ParsedEvent>,
}

pub struct ParsedInstruction {
    pub name: String,
    pub docs: Option<Vec<String>>,
    pub accounts: Vec<IdlAccountItem>,
    pub args: Vec<IdlField>,
    pub returns: Option<IdlType>,
}

pub struct ParsedAccount {
    pub name: String,
    pub docs: Option<Vec<String>>,
    pub type_info: String,
}

pub struct ParsedEvent {
    pub name: String,
    pub fields: Vec<IdlField>,
}

pub struct ParsedType {
    pub name: String,
    pub docs: Option<Vec<String>>,
    pub type_kind: TypeKind,
}

pub enum TypeKind {
    Struct(Vec<IdlField>),
    Enum(Vec<IdlEnumVariant>),
    Alias(IdlType),
}

/// Parse an Anchor program from the given path
pub fn parse_program(program_path: &Path) -> Result<ParsedProgram> {
    // Collect all Rust files in the program directory
    let rust_files = collect_rust_files(program_path)?;
    
    // Process each file and collect program elements
    let mut program_name = String::new();
    let mut docs = None;
    let mut instructions = Vec::new();
    let mut accounts = Vec::new();
    let mut types = Vec::new();
    let mut events = Vec::new();
    
    for file_path in rust_files {
        let file_content = fs::read_to_string(&file_path)?;
        let syntax = parse_file(&file_content)?;
        
        // Parse module-level attributes for program info
        for attr in &syntax.attrs {
            if is_program_attribute(attr) {
                program_name = extract_program_name(attr)?;
                docs = extract_docs(&syntax.attrs);
            }
        }
        
        // Parse items in the file
        for item in syntax.items {
            match item {
                Item::Fn(func) => {
                    if has_instruction_attribute(&func.attrs) {
                        instructions.push(parse_instruction(func)?);
                    }
                },
                Item::Struct(strct) => {
                    if has_account_attribute(&strct.attrs) {
                        accounts.push(parse_account(strct)?);
                    } else if has_event_attribute(&strct.attrs) {
                        events.push(parse_event(strct)?);
                    } else {
                        types.push(parse_struct_type(strct)?);
                    }
                },
                Item::Enum(enm) => {
                    types.push(parse_enum_type(enm)?);
                },
                _ => {}
            }
        }
    }
    
    if program_name.is_empty() {
        return Err(anyhow!("No program found in the specified path"));
    }
    
    Ok(ParsedProgram {
        name: program_name,
        docs,
        instructions,
        accounts,
        types,
        events,
    })
}

/// Collect all Rust files in the program directory
fn collect_rust_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let mut sub_files = collect_rust_files(&path)?;
                result.append(&mut sub_files);
            } else if let Some(extension) = path.extension() {
                if extension == "rs" {
                    result.push(path);
                }
            }
        }
    } else if dir.extension().map_or(false, |ext| ext == "rs") {
        result.push(dir.to_path_buf());
    }
    
    Ok(result)
}

/// Check if an attribute is a program attribute
fn is_program_attribute(attr: &Attribute) -> bool {
    attr.path().segments.last()
        .map_or(false, |seg| seg.ident == "program")
}

/// Extract program name from a program attribute
fn extract_program_name(attr: &Attribute) -> Result<String> {
    // Parse the attribute to extract the program name
    // This is simplified and would need actual parsing in a real implementation
    Ok("my_program".to_string())
}

/// Extract documentation comments from attributes
fn extract_docs(attrs: &[Attribute]) -> Option<Vec<String>> {
    let docs: Vec<String> = attrs.iter()
        .filter(|attr| attr.path().segments.last().map_or(false, |seg| seg.ident == "doc"))
        .filter_map(|attr| {
            if let Ok(syn::Meta::NameValue(name_value)) = attr.meta.clone().require_name_value() {
                if let syn::Expr::Lit(expr_lit) = name_value.value {
                    if let syn::Lit::Str(lit_str) = expr_lit.lit {
                        return Some(lit_str.value());
                    }
                }
            }
            None
        })
        .collect();
    
    if docs.is_empty() {
        None
    } else {
        Some(docs)
    }
}

/// Check if a function has an instruction attribute
fn has_instruction_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| 
        attr.path().segments.last().map_or(false, |seg| seg.ident == "instruction")
    )
}

/// Parse an instruction from a function
fn parse_instruction(func: ItemFn) -> Result<ParsedInstruction> {
    // This would be a complex function to parse the function signature and body
    // to extract instruction information. Simplified for this example.
    
    let name = func.sig.ident.to_string();
    let docs = extract_docs(&func.attrs);
    
    // In a real implementation, you would parse:
    // - Function arguments to get instruction args
    // - #[account] attributes to get accounts
    // - Return type if present
    
    Ok(ParsedInstruction {
        name,
        docs,
        accounts: Vec::new(), // Would be populated in real implementation
        args: Vec::new(),     // Would be populated in real implementation
        returns: None,        // Would be populated in real implementation
    })
}

/// Check if a struct has an account attribute
fn has_account_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| 
        attr.path().segments.last().map_or(false, |seg| seg.ident == "account")
    )
}

/// Parse an account from a struct
fn parse_account(strct: ItemStruct) -> Result<ParsedAccount> {
    let name = strct.ident.to_string();
    let docs = extract_docs(&strct.attrs);
    
    // In a real implementation, you would extract the full account structure
    
    Ok(ParsedAccount {
        name,
        docs,
        type_info: name.clone(), // In a full implementation, this would be the fully qualified type
    })
}

/// Check if a struct has an event attribute
fn has_event_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| 
        attr.path().segments.last().map_or(false, |seg| seg.ident == "event")
    )
}

/// Parse an event from a struct
fn parse_event(strct: ItemStruct) -> Result<ParsedEvent> {
    let name = strct.ident.to_string();
    
    // Parse the struct fields to get event fields
    let fields = parse_struct_fields(&strct.fields)?;
    
    Ok(ParsedEvent {
        name,
        fields,
    })
}

/// Parse a struct type
fn parse_struct_type(strct: ItemStruct) -> Result<ParsedType> {
    let name = strct.ident.to_string();
    let docs = extract_docs(&strct.attrs);
    
    // Parse the struct fields
    let fields = parse_struct_fields(&strct.fields)?;
    
    Ok(ParsedType {
        name,
        docs,
        type_kind: TypeKind::Struct(fields),
    })
}

/// Parse fields from a struct
fn parse_struct_fields(fields: &syn::Fields) -> Result<Vec<IdlField>> {
    let mut result = Vec::new();
    
    match fields {
        syn::Fields::Named(named_fields) => {
            for field in &named_fields.named {
                if let Some(ident) = &field.ident {
                    let name = ident.to_string();
                    let docs = extract_docs(&field.attrs);
                    
                    // In a real implementation, you would parse the field type
                    // to convert it to an IdlType
                    let ty = crate::idl::generate_type_signature(&field.ty)?;
                    
                    result.push(IdlField {
                        name,
                        docs,
                        ty,
                    });
                }
            }
        },
        syn::Fields::Unnamed(_) => {
            // Handle tuple structs if needed
            return Err(anyhow!("Tuple structs are not supported"));
        },
        syn::Fields::Unit => {
            // Unit structs have no fields
        },
    }
    
    Ok(result)
}

/// Parse an enum type
fn parse_enum_type(enm: ItemEnum) -> Result<ParsedType> {
    let name = enm.ident.to_string();
    let docs = extract_docs(&enm.attrs);
    
    // Parse the enum variants
    let mut variants = Vec::new();
    
    for variant in &enm.variants {
        let variant_name = variant.ident.to_string();
        let variant_docs = extract_docs(&variant.attrs);
        
        match &variant.fields {
            syn::Fields::Named(named_fields) => {
                // Parse named fields for this variant
                let fields = parse_struct_fields(&syn::Fields::Named(named_fields.clone()))?;
                
                variants.push(IdlEnumVariant {
                    name: variant_name,
                    docs: variant_docs,
                    fields: Some(fields),
                });
            },
            syn::Fields::Unnamed(unnamed_fields) => {
                // Handle tuple variants if needed
                return Err(anyhow!("Tuple enum variants are not supported"));
            },
            syn::Fields::Unit => {
                // Unit variant has no fields
                variants.push(IdlEnumVariant {
                    name: variant_name,
                    docs: variant_docs,
                    fields: None,
                });
            },
        }
    }
    
    Ok(ParsedType {
        name,
        docs,
        type_kind: TypeKind::Enum(variants),
    })
}