//! Validation Test Suite
//!
//! This test runner includes all validation tests for FrankenBrowser.
//!
//! Tests included:
//! - FEAT-030: Basic navigation validation (google.com)
//! - FEAT-031: Performance target (page load < 3s)
//! - FEAT-032: Memory target (< 500MB per tab)
//! - FEAT-033: Top 10 websites validation
//! - FEAT-034: Stability validation (1 hour session)

mod validation;
