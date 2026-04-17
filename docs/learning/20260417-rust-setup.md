# Rust Setup for Kimono

## What We're Installing

1. **rustup** — Rust's toolchain manager (like `nvm` for Node). Manages Rust versions, components, and targets.
2. **rustc** — the Rust compiler itself
3. **cargo** — package manager + build tool (like npm/yarn but also handles compilation)
4. **rust-std** — the standard library
5. **rust-analyzer** — LSP server for IDE support (optional but recommended)

## Installation Steps

### Step 1: Install rustup + stable toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

What this does:
- Downloads the `rustup` installer from the official Rust site
- `-y` accepts defaults non-interactively
- Installs the **stable** toolchain (latest stable Rust release)
- Adds `~/.cargo/bin` to your PATH via `~/.zshenv` or `~/.zprofile`
- Creates `~/.rustup/` (toolchain storage) and `~/.cargo/` (package cache + binaries)

After install, `~/.cargo/bin` contains: `rustc`, `cargo`, `rustup`, `rustfmt`, `clippy`, etc.

### Step 2: Source the cargo environment

```bash
source "$HOME/.cargo/env"
```

This adds `~/.cargo/bin` to the current shell's PATH. New terminal sessions pick it up automatically.

### Step 3: Verify the installation

```bash
rustc --version    # Should print: rustc 1.x.x (hash date)
cargo --version    # Should print: cargo 1.x.x (hash date)
rustup --version   # Should print: rustup 1.x.x (hash date)
```

### Step 4: (Optional) Install rust-analyzer for IDE support

```bash
rustup component add rust-analyzer
```

If you use VS Code, install the "rust-analyzer" extension. It provides:
- Autocomplete, go-to-definition, inline type hints
- Real-time error checking (no need to `cargo build` to see errors)
- Refactoring tools

## What Gets Created

```
~/.rustup/                    # Toolchain installations
  toolchains/
    stable-aarch64-apple-darwin/   # The actual compiler + stdlib
~/.cargo/
  bin/                        # Binaries: rustc, cargo, rustup, rustfmt, clippy
  registry/                   # Downloaded crate sources (like node_modules cache)
  env                         # Shell script that adds bin/ to PATH
```

## Key Commands After Install

```bash
rustup update              # Update to latest stable Rust
rustup show                # Show installed toolchains
cargo new myproject        # Create a new Rust project
cargo init                 # Initialize Rust in existing directory
cargo build                # Compile (debug mode)
cargo build --release      # Compile (optimized)
cargo run                  # Build + run
cargo test                 # Run tests
cargo fmt                  # Format code (like prettier)
cargo clippy               # Lint (like eslint)
cargo add <crate>          # Add a dependency (like npm install)
```

## Uninstalling (if ever needed)

```bash
rustup self uninstall      # Removes everything: rustup, cargo, rustc, ~/.rustup, ~/.cargo
```
