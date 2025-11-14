# Browser Shell

**Type**: library  
**Level**: 3  
**Version**: 0.1.0

## Responsibility

Window and tab management, UI shell

## Dependencies

- `shared_types`
- `message_bus`
- `config_manager`
- `network_stack`
- `adblock_engine`
- `browser_core`
- `webview_integration`

## Structure

```
browser_shell/
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

- **Estimated tokens**: 14,000
- **Optimal limit**: 70,000
- **Current status**: Well within limits ✅
