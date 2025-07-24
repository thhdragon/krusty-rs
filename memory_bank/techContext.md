# Tech Context

## Technologies Used
- Rust (async/await, tokio or async-std)
- Axum (web API)
- TOML/JSON for configuration
- thiserror, anyhow for error handling
- Unit/integration testing with Rust test framework

## Development Setup
1. Install Rust toolchain (rustup, cargo)
2. Clone repository and run `cargo build`/`cargo test`
3. Edit configuration files in TOML/JSON as needed
4. Run main application via `cargo run` (see `main.rs`)
5. Use provided test harnesses and simulation tools for validation

## Technical Constraints
- All motion and hardware control must be real-time safe and async
- Web API must be secure and extensible
- Hardware abstraction must support modular addition of peripherals
- Error handling must be robust and contextualized

## Dependencies
- tokio or async-std (async runtime)
- axum (web API)
- thiserror, anyhow (error handling)
- serde, toml, json (config parsing)
- [see Cargo.toml for full list]
