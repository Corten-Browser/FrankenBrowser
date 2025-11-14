# Browser Shell Component

## ⚠️ CRITICAL: READ PROJECT CLAUDE.MD FIRST

**Before starting work, read:**
```bash
cat ../../CLAUDE.md
```

This file contains component-specific guidance. The project CLAUDE.md contains:
- Orchestration system overview
- Quality standards (TDD, 80%+ coverage, etc.)
- Version control restrictions (NEVER change to 1.0.0)
- Breaking changes policy (encouraged in 0.x.x)
- Component interaction rules
- Token budget guidelines

## Component Overview

**Name**: browser_shell  
**Type**: library  
**Level**: 3 (dependency level)  
**Description**: Window and tab management, UI shell  
**Token Budget**: 14,000 / 70,000 (optimal)

## Your Boundaries

**Your Working Directory**: `components/browser_shell/`

**You CAN:**
- ✅ Work in `components/browser_shell/` directory
- ✅ Import from dependencies' public APIs
- ✅ Read contracts from `../../contracts/`
- ✅ Use cargo workspace dependencies

**You CANNOT:**
- ❌ Modify other components' files
- ❌ Access private implementation of other components
- ❌ Change project version to 1.0.0
- ❌ Declare system "production ready"

## Dependencies

This component depends on:
- `shared_types` (import via Cargo workspace)
- `message_bus` (import via Cargo workspace)
- `config_manager` (import via Cargo workspace)
- `network_stack` (import via Cargo workspace)
- `adblock_engine` (import via Cargo workspace)
- `browser_core` (import via Cargo workspace)
- `webview_integration` (import via Cargo workspace)

**Import Example**:
```rust
  use shared-types::*;  // Import from shared_types
  use message-bus::*;  // Import from message_bus
  use config-manager::*;  // Import from config_manager
  use network-stack::*;  // Import from network_stack
  use adblock-engine::*;  // Import from adblock_engine
  use browser-core::*;  // Import from browser_core
  use webview-integration::*;  // Import from webview_integration
```

## Project Context

**Project Root**: `/home/user/FrankenBrowser/`  
**Specification**: `/home/user/FrankenBrowser/frankenstein-browser-specification.md`  
**Architecture**: `/home/user/FrankenBrowser/ARCHITECTURE.md`  
**Language**: Rust (edition 2021)  
**Package Manager**: Cargo  
**Version**: 0.1.0 (pre-release)


## Rust-Specific Guidelines

### Code Organization
```
src/
├── lib.rs        # Public API exports
├── types.rs      # Type definitions
├── errors.rs     # Error types
└── impl/         # Implementation details
```

### Public API Pattern
```rust
// src/lib.rs - Public exports
pub mod types;
pub mod errors;

// Re-export main types
pub use types::{PublicType, PublicEnum};
pub use errors::{Error, Result};

// Keep implementation private
mod impl;
```

### Testing Requirements
```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt --check
```

### Quality Gates
- ✅ All tests pass (100%)
- ✅ Test coverage ≥ 80%
- ✅ No clippy warnings
- ✅ Formatted with cargo fmt
- ✅ No unsafe code without justification



## Implementation Requirements

### UI Shell Components

1. **BrowserShell Struct**
   - Window management using tao
   - Tab management
   - UI integration
   - Event loop

2. **Window Management**
   - Create windows
   - Handle window events
   - Menu bar
   - Keyboard shortcuts

3. **Tab Manager**
   - Create/close/switch tabs
   - Tab state tracking
   - Active tab management

4. **UI Components**
   - URL bar
   - Navigation buttons
   - Tab bar
   - Status bar
   - Context menus


## Test-Driven Development (MANDATORY)

### TDD Workflow
1. **RED**: Write failing test first
2. **GREEN**: Write minimal code to pass
3. **REFACTOR**: Improve code quality

### Example TDD Cycle
```rust
// Step 1: RED - Write failing test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_feature() {
        let component = Component::new();
        let result = component.new_feature();
        assert_eq!(result, expected_value);
    }
}

// Step 2: GREEN - Implement to pass
pub fn new_feature(&self) -> Result<Value> {
    // Minimal implementation
    Ok(expected_value)
}

// Step 3: REFACTOR - Improve
// Add error handling, edge cases, etc.
```

### Required Test Coverage
- ✅ Unit tests for all public functions
- ✅ Integration tests for workflows
- ✅ Coverage ≥ 80%
- ✅ All tests must pass (100%)

## Token Budget Monitoring

**CRITICAL**: Monitor component size continuously.

```bash
# Check size before committing
find src tests -type f -name "*.rs" | xargs wc -l

# If > 7,000 lines: Note in commit
# If > 9,000 lines: Alert orchestrator  
# If > 11,000 lines: STOP - split needed
```

**Current Estimate**: 14,000 tokens  
**Status**: ✅ Well within limits

## Quality Checklist

Before marking component complete:
- [ ] All tests pass (100% pass rate)
- [ ] Test coverage ≥ 80%
- [ ] TDD compliance (git history shows Red-Green-Refactor)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt --check`)
- [ ] Public APIs documented
- [ ] No unsafe code without justification
- [ ] Contract compliance (if applicable)
- [ ] Component size < 70,000 tokens

## Completion Criteria

**This component is complete when:**
1. All required functionality implemented per specification
2. All tests passing (unit + integration)
3. Test coverage ≥ 80%
4. Quality gates passing
5. Documentation complete
6. Git commits show TDD pattern

## Getting Help

If you encounter:
- **Specification ambiguity**: Make reasonable technical decisions (0.x.x allows breaking changes)
- **Dependency issues**: Check other component's public API in their src/lib.rs
- **Size concerns**: Alert orchestrator if approaching 70k tokens
- **Architecture questions**: Refer to ARCHITECTURE.md

## Commit Message Format

```
[browser_shell] Brief description

- What changed
- Why it changed
- Tests added/updated

Coverage: X%
Size: ~Y tokens
```

## Resources

- **Project Spec**: `../../frankenstein-browser-specification.md`
- **Architecture**: `../../ARCHITECTURE.md`
- **Orchestrator Instructions**: `../../CLAUDE.md`
- **Component Manifest**: `./component.yaml`

---

**Remember**: You're building a piece of a larger system. Focus on your component, use dependencies through public APIs, and maintain high quality standards. The orchestrator will handle integration and system-wide testing.
