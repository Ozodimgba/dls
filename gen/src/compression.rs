use anchor_syn::idl::types::Idl;
use anyhow::Result;
use flate2::{write::ZlibEncoder, read::ZlibDecoder, Compression};
use std::io::{Read, Write};

/// Compress IDL data using zlib
pub fn compress_idl(idl: &Idl) -> Result<Vec<u8>> {
    let json_bytes = serde_json::to_vec(idl)?;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&json_bytes)?;
    encoder.finish().map_err(Into::into)
}

/// Decompress IDL data
pub fn decompress_idl(data: &[u8]) -> Result<Idl> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    let idl: Idl = serde_json::from_slice(&decompressed)?;
    Ok(idl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_syn::idl::types::{Idl, IdlInstruction};

    #[test]
    fn test_compression() {
        // Create a sample IDL
        let idl = Idl {
            version: "0.1.0".to_string(),
            name: "test_program".to_string(),
            instructions: vec![
                IdlInstruction {
                    name: "test_instruction".to_string(),
                    accounts: vec![],
                    args: vec![],
                    returns: None,
                    docs: None,
                }
            ],
            accounts: vec![],
            types: None,
            events: None,
            errors: None,
            constants: None,
            docs: None,
            metadata: None,
        };

        // Test compression and decompression
        let compressed = compress_idl(&idl).unwrap();
        let decompressed = decompress_idl(&compressed).unwrap();

        assert_eq!(idl.name, decompressed.name);
        assert_eq!(idl.version, decompressed.version);
        assert_eq!(idl.instructions.len(), decompressed.instructions.len());
        assert_eq!(idl.instructions[0].name, decompressed.instructions[0].name);
    }
}