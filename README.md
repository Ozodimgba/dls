# Anchor IDL CLI

A lightweight command-line tool for generating and converting Solana Anchor IDLs without requiring the full Anchor CLI installation.

## Features

- **Build IDLs** from Solana Anchor programs
- **Convert IDLs** from legacy formats to the current specification
- **Validate IDLs** against the specification
- **Display Instruction Details** to understand program interfaces
- Standalone operation without requiring the full Anchor toolchain

## Installation

```bash
cargo install --path .
```

Or install from crate.io:

```bash
cargo install dls-anchor
```

## Usage

### Building an IDL

```bash
# Basic usage (in the program directory)
dls-anchor build

# Specify program path and output file
dls-anchor build --path /path/to/my/program --output my_program_idl.json

# Skip linting and docs generation
dls-anchor build --skip-lint --no-docs

# Disable account resolution
dls-anchor build --no-resolution

# Generate a legacy format IDL (pre Anchor v0.30)
dls-anchor build --legacy
```

### Converting an IDL

```bash
# Convert from a legacy format
dls-anchor convert --input legacy_idl.json --output converted_idl.json
```

### Validating an IDL

```bash
# Validate an IDL against the specification
dls-anchor validate --input my_program_idl.json
```

### Viewing Program Instructions

```bash
# Show detailed information about all instructions in the program
dls-anchor instructions --input my_program_idl.json

# Show only instruction names for a quick overview
dls-anchor instructions --input my_program_idl.json --names-only
```

### Verbose Mode

Add the `--verbose` flag to any command for detailed logging:

```bash
dls-anchor --verbose build
```

## Requirements

- Rust toolchain (with nightly support)
- Appropriate Solana program dependencies

## How It Works

The CLI leverages the `anchor-lang-idl` crate, which is extracted from the Anchor framework. It provides:

1. IDL generation via program analysis
2. Legacy IDL format conversion
3. Validation against the specification
4. Human-readable instruction display

## License

Apache License 2.0