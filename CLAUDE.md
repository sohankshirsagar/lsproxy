# LSPROXY Development Guide

## Build & Run Commands
- Build: `./scripts/build.sh` (Docker-based)
- Run: `./scripts/run.sh $WORKSPACE_PATH` (Docker-based)
- Format: `./scripts/fmt.sh`
- Tests: `./scripts/test.sh`
- Run single test: `./scripts/test.sh <test_name>`
- Code coverage: `./scripts/coverage_test.sh`

## Code Style Guidelines
- `cargo fmt`
- `cargo fix --allow-dirty`

## Project Structure
- `/lsproxy/src/`: Main Rust codebase
- `/lsproxy/src/lsp/`: Language Server Protocol integration
- `/lsproxy/src/handlers/`: REST API endpoint handlers
- `/lsproxy/src/ast_grep/`: AST-grep integration for symbol identification
