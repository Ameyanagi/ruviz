# Code Style and Conventions

## Formatting (rustfmt.toml)
- Max width: 100 characters
- 4 spaces (no tabs)
- Unix newlines
- Import reordering enabled
- Rust 2024 edition

## Linting (.clippy.toml)
- MSRV: 1.82
- Performance-focused lints enabled
- Breaking API changes allowed during development

## Naming Conventions
- Snake_case for modules, functions, variables
- PascalCase for structs, enums, traits
- SCREAMING_SNAKE_CASE for constants
- Descriptive names preferred over abbreviations

## Code Organization
- Public API in `src/lib.rs`
- Module structure follows feature separation
- Tests co-located with implementation when possible
- Integration tests in separate `tests/` directory