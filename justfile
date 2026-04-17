# kimono dev tasks — run `just` to list, `just <recipe>` to run one

# Default recipe: show the list
default:
    @just --list

# Run the full test suite (unit + integration)
test:
    cargo test

# Run only unit tests (fast)
test-unit:
    cargo test --bin kimono

# Run only the end-to-end integration test
test-e2e:
    cargo test --test e2e_test

# Build the release binary (optimized)
build:
    cargo build --release

# Fast compile check — no codegen
check:
    cargo check

# Format code with rustfmt
fmt:
    cargo fmt

# Lint with clippy
lint:
    cargo clippy --all-targets

# Install kimono to ~/.cargo/bin (runs tests first)
install: test
    cargo install --path . --quiet
    @echo ""
    @echo "✓ Installed: $(which kimono)"
    @kimono --version

# Install without running tests (faster iteration)
install-fast:
    cargo install --path . --quiet
    @echo "✓ Installed: $(which kimono)"

# Clean build artifacts
clean:
    cargo clean
