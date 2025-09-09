# Suggested Commands

## Development Commands
- `cargo build` - Build the project
- `cargo test` - Run all tests
- `cargo test --lib` - Run unit tests only
- `cargo test integration` - Run integration tests
- `cargo test contract` - Run contract tests
- `cargo clippy` - Run linter
- `cargo fmt` - Format code
- `cargo bench` - Run performance benchmarks
- `cargo doc --open` - Generate and open documentation

## Testing Strategy
- Unit tests: `tests/unit/`
- Integration tests: `tests/integration/` 
- Contract tests: `tests/contract/`
- Visual tests: `tests/visual/`
- Benchmarks: `benches/`

## Build Variants
- `cargo build --release` - Optimized build
- `cargo build --features full` - All features enabled
- `cargo build --no-default-features` - Minimal build