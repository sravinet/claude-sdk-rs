# Testing Guide

This document explains how to run different categories of tests in the Claude SDK Rust project.

## Quick Development Testing

For fast feedback during development:

```bash
# Run fast unit tests only (recommended for development)
cargo test --lib

# Run tests with specific features
cargo test --lib --features cli
```

**Expected time: ~2 seconds**

## Full Test Suite

For comprehensive testing before commits/CI:

```bash
# Run all tests including slow ones
cargo test --lib -- --ignored

# Run all tests (fast + slow)
cargo test --lib -- --include-ignored
```

**Expected time: 3+ minutes**

## Test Categories

### Fast Tests (Default)
- ✅ Unit tests
- ✅ Integration tests with mocked data
- ✅ Core functionality tests

### Slow Tests (Ignored by Default)
- 🐌 Performance benchmarks (`#[ignore = "slow performance test"]`)
- 🐌 Load testing scenarios (`#[ignore = "slow performance test"]`) 
- 🐌 Stress tests with large data volumes (`#[ignore = "slow stress test"]`)
- 🐌 Cache performance tests (`#[ignore = "slow cache test"]`)
- 🐌 Background task tests (`#[ignore = "slow background task test"]`)

## Running Specific Test Categories

```bash
# Run only performance tests
cargo test --lib -- --ignored "slow performance test"

# Run only cache tests  
cargo test --lib -- --ignored "slow cache test"

# Run only stress tests
cargo test --lib -- --ignored "slow stress test"
```

## CI/CD Recommendations

- **Pull Requests**: Run `cargo test --lib` (fast tests only)
- **Main Branch**: Run `cargo test --lib -- --include-ignored` (all tests)
- **Nightly Builds**: Include performance regression testing

## Why This Organization?

Fast tests provide immediate feedback during development, while slow tests ensure comprehensive performance validation without blocking the development workflow.

Performance tests include realistic data generation, actual I/O operations, and time-based measurements that are essential for quality but too slow for rapid iteration.