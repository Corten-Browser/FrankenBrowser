# FrankenBrowser API Contracts

This directory contains API contract specifications for all components in the FrankenBrowser project.

## Purpose

Contracts define the public APIs that components expose and depend on. They serve as:
- **Documentation**: Clear specification of component interfaces
- **Validation**: Automated checks for API compliance
- **Integration**: Reference for component communication

## Contract Files

| Component | File | Description |
|-----------|------|-------------|
| shared_types | shared_types.yaml | Common types and messages |
| message_bus | message_bus.yaml | Message bus traits and structs |
| browser_core | browser_core.yaml | Browser engine API |

## Contract Format

Contracts are written in YAML and include:
- **Component metadata**: name, version, dependencies
- **Exports**: structs, enums, traits, functions
- **Method signatures**: Rust type signatures
- **Error conditions**: What can go wrong
- **Usage examples**: How to use the API

## Validation

Before integration testing, all components must pass contract compliance:

```bash
python orchestration/contract_enforcer.py check <component-name>
```

This verifies:
- All required types are implemented
- Method signatures match
- Error handling is present
- Documentation is complete

## Usage in Development

1. **Before implementing**: Read the contract to understand the API
2. **During implementation**: Implement all required methods
3. **Before completion**: Validate contract compliance
4. **During integration**: Use contract tests

## Contract Evolution

During 0.x.x (pre-release):
- ✅ Breaking changes allowed and encouraged
- ✅ API refinement based on implementation experience
- ✅ Update contract before changing implementation

After 1.0.0 (if reached):
- ❌ Breaking changes require major version bump
- ✅ Backward compatibility required
- ✅ Deprecation process for API changes
