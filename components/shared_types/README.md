# Shared Types

**Type**: library  
**Level**: 0  
**Version**: 0.1.0

## Responsibility

Common types, messages, and error definitions

## Dependencies

None (base component)

## Structure

```
shared_types/
├── CLAUDE.md          # Component-specific instructions for Claude Code
├── README.md          # This file
├── Cargo.toml         # Rust package configuration
├── component.yaml     # Component manifest
├── src/               # Source code
│   └── lib.rs         # Main library file
└── tests/             # Tests (unit, integration)
    ├── unit/
    └── integration/
```

## Usage

This component is part of the FrankenBrowser project and is built using Cargo:

```bash
# Build
cargo build

# Test
cargo test

# Run clippy
cargo clippy

# Format
cargo fmt
```

## Development

See CLAUDE.md for detailed development instructions, quality standards, and TDD requirements.

## Token Budget

- **Estimated tokens**: 5,000
- **Optimal limit**: 70,000
- **Current status**: Well within limits ✅
