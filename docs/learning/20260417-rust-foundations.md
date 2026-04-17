# Rust Foundations — What You Need Before We Start

This doc covers the Rust concepts you'll see in the first files of kimono. Read this before diving into the code.

---

## Terminology Cheat Sheet

| Rust Term | Closest JS/TS Equivalent | What It Actually Means |
|-----------|--------------------------|------------------------|
| **crate** | npm package | A Rust library or binary. `kimono` is a crate. `clap`, `serde` are crates we depend on. |
| **Cargo** | npm/yarn/pnpm | Rust's package manager AND build tool. `Cargo.toml` = `package.json`. `cargo build` = `npm run build`. `cargo run` = `node index.js`. |
| **Cargo.toml** | package.json | Project metadata + dependencies. |
| **Cargo.lock** | yarn.lock / pnpm-lock.yaml | Exact dependency versions. Committed for binaries, not for libraries. |
| **`mod`** | `import` / directory index | Declares a module (a file or directory of code). `mod cli;` tells Rust "there's a `cli.rs` file or `cli/mod.rs` directory, include it." |
| **`use`** | `import { X } from` | Brings a name into scope. `use clap::Parser;` = `import { Parser } from 'clap'`. |
| **`pub`** | `export` | Makes something visible outside its module. Without `pub`, it's private. |
| **`struct`** | TypeScript `interface` / `type` | A data structure with named fields. `struct Repo { name: String, branch: String }` |
| **`enum`** | TypeScript union type | A type that can be one of several variants. Way more powerful than TS unions — each variant can hold data. |
| **`impl`** | class methods | Attaches methods to a struct or enum. `impl Repo { fn name(&self) -> &str { ... } }` |
| **`trait`** | TypeScript interface (with methods) | A set of methods a type can implement. Like a contract. `Display`, `Debug`, `Clone` are common traits. |
| **`derive`** | decorators (loosely) | Auto-implements traits for a struct. `#[derive(Debug, Clone)]` = "auto-generate Debug and Clone for this type." |
| **`fn`** | `function` | Function declaration. |
| **`let`** | `const` | Variable binding (immutable by default). |
| **`let mut`** | `let` | Mutable variable binding. |
| **`&`** | (no equivalent) | A reference / borrow. See Ownership section below. |
| **`String`** | `string` (heap) | Owned, growable string on the heap. |
| **`&str`** | string literal / string view | Borrowed reference to a string. Think "I'm looking at a string someone else owns." |
| **`Vec<T>`** | `Array<T>` | Growable array/list. `Vec<String>` = `string[]`. |
| **`HashMap<K, V>`** | `Map<K, V>` / `Record<K, V>` | Key-value map. |
| **`Option<T>`** | `T \| undefined` | Either `Some(value)` or `None`. Rust has no `null` or `undefined`. |
| **`Result<T, E>`** | try/catch (but as a type) | Either `Ok(value)` or `Err(error)`. This is how Rust handles errors — no exceptions. |
| **`?` operator** | `await` (kinda) | Unwraps a `Result` or `Option` — returns the value if `Ok`/`Some`, returns the error early if `Err`/`None`. |
| **`match`** | `switch` (on steroids) | Pattern matching. Must handle all cases — the compiler enforces it. |
| **`unwrap()`** | `!` non-null assertion | "I'm sure this is `Some`/`Ok`, crash if I'm wrong." Use sparingly — `?` is better. |
| **`clone()`** | spread operator / structuredClone | Makes a deep copy. In Rust you need to be explicit about copies. |

---

## The Big Concept: Ownership

This is the thing that makes Rust different from every language you've used. It's also the thing that lets Rust have no garbage collector while being memory-safe.

### The Rules

1. Every value has exactly **one owner** (a variable).
2. When the owner goes out of scope, the value is **dropped** (freed).
3. You can **borrow** a value (get a reference) without taking ownership.

### In Practice

```rust
fn main() {
    let name = String::from("kimono");  // `name` owns this string

    print_name(&name);    // Borrow — `name` still owns it
    print_name(&name);    // Can borrow again, no problem

    take_name(name);      // Ownership MOVES to `take_name`
    // print_name(&name); // ERROR: `name` was moved, can't use it anymore
}

fn print_name(n: &str) {       // Borrows a string (read-only reference)
    println!("{}", n);
}

fn take_name(n: String) {      // Takes ownership of the string
    println!("I own: {}", n);
}   // `n` is dropped here, memory freed
```

**Why this matters for kimono**: When we pass config data around, we'll use `&` references most of the time (borrowing), and `clone()` when we need a separate copy. The compiler tells you when you get it wrong — that's the point.

### The JS Comparison

In JS, everything is either a primitive (copied) or an object (shared reference, garbage collected). You never think about who owns what. In Rust, the compiler tracks ownership and tells you at compile time if you're using something after it was moved or freed. No runtime surprises.

---

## Error Handling: Result and `?`

Rust has no `try/catch`. Instead, functions that can fail return `Result<T, E>`:

```rust
use std::fs;

// This function can fail — it returns Result
fn read_config() -> Result<String, std::io::Error> {
    let content = fs::read_to_string(".kimono/config.yml")?;  // ? = "return error if this fails"
    Ok(content)  // Wrap the success value in Ok()
}

// Using it:
fn main() {
    match read_config() {
        Ok(content) => println!("Config: {}", content),
        Err(e) => eprintln!("Failed to read config: {}", e),
    }
}
```

The `?` operator is the key ergonomic tool. It means "if this is an error, return it from the current function immediately." It's like auto-propagating exceptions, but explicit in the type signature.

In kimono, we use the `anyhow` crate to simplify error handling:

```rust
use anyhow::{Result, Context};

fn read_config() -> Result<String> {
    let content = fs::read_to_string(".kimono/config.yml")
        .context("Failed to read .kimono/config.yml")?;
    Ok(content)
}
```

`anyhow::Result<T>` = `Result<T, anyhow::Error>` where `anyhow::Error` can hold any error type. `.context()` adds a human-readable message to the error chain.

---

## Structs + Derive: How We Define Config

This is what our config types will look like:

```rust
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub workspace: Workspace,
    pub repos: HashMap<String, Repo>,
}

#[derive(Debug, Deserialize)]
pub struct Workspace {
    pub name: String,
    #[serde(default = "default_apps_dir")]
    pub apps_dir: String,
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,
}

fn default_apps_dir() -> String { "apps".to_string() }
fn default_worktree_dir() -> String { ".worktrees".to_string() }

#[derive(Debug, Deserialize)]
pub struct Repo {
    pub remote: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    pub description: Option<String>,
    #[serde(default)]
    pub tech: Vec<String>,
    pub package_manager: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub commands: HashMap<String, String>,
}

fn default_branch() -> String { "main".to_string() }
```

**Breaking this down:**

- `#[derive(Debug, Deserialize)]` — auto-generates two trait implementations:
  - `Debug` lets you `println!("{:?}", config)` for debugging
  - `Deserialize` (from serde) lets serde_yaml parse YAML directly into this struct
- `pub` — makes the field accessible from other modules
- `Option<String>` — this field might not be present in the YAML (it's optional)
- `Vec<String>` — an array of strings
- `HashMap<String, String>` — a key-value map (like `Record<string, string>` in TS)
- `#[serde(default)]` — if the field is missing from YAML, use the type's default (empty vec, empty map, etc.)
- `#[serde(default = "function_name")]` — if missing, call this function to get the default value

**This is the payoff of choosing Rust**: with ~40 lines of struct definitions, we get YAML parsing, validation, default values, and type safety. If the YAML has a wrong type or missing required field, serde gives a clear error message pointing at the problem. In bash, you'd need hundreds of lines of validation code.

---

## Clap: How We Define the CLI

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kimono", about = "Multi-repo aggregator for AI-assisted development")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new kimono workspace
    Init,
    /// Clone repos from config
    Clone {
        /// Specific repos to clone (default: all)
        repos: Vec<String>,
    },
    /// Show working tree status across repos
    Status {
        /// Specific repos to check
        repos: Vec<String>,
    },
    /// Worktree management
    Wt {
        #[command(subcommand)]
        command: WtCommands,
    },
}

#[derive(Subcommand)]
pub enum WtCommands {
    /// Create a worktree
    Add {
        repo: String,
        branch: String,
        #[arg(long)]
        new: bool,
    },
    /// Cross-repo feature worktrees
    Feature {
        name: String,
        repos: Vec<String>,
        #[arg(long)]
        new: bool,
        #[arg(long)]
        remove: bool,
    },
}
```

**What's happening:**

- `#[derive(Parser)]` / `#[derive(Subcommand)]` — clap's derive macros auto-generate the CLI parser from your struct/enum definitions
- `/// comment` — doc comments become the help text (`kimono --help`)
- `#[arg(long)]` — makes a field a `--flag` instead of a positional argument
- `Vec<String>` on a positional = variadic args (`kimono clone backend frontend`)
- Enums model subcommands perfectly — each variant is a subcommand with its own args

From this definition, clap auto-generates:
- `kimono --help` with all commands listed
- `kimono wt --help` with sub-subcommands
- `kimono wt feature --help` with all flags
- Colored output, error messages for wrong args, shell completions

**The main function ties it together:**

```rust
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();  // Parse command-line args into our struct

    match cli.command {
        Commands::Init => cmd::init::run()?,
        Commands::Clone { repos } => cmd::clone::run(&repos)?,
        Commands::Status { repos } => cmd::status::run(&repos)?,
        Commands::Wt { command } => match command {
            WtCommands::Add { repo, branch, new } => cmd::wt::add::run(&repo, &branch, new)?,
            WtCommands::Feature { name, repos, new, remove } => {
                cmd::wt::feature::run(&name, &repos, new, remove)?
            }
        },
    }

    Ok(())
}
```

`main() -> anyhow::Result<()>` means main can return errors. If any `?` propagates an error up to main, Rust prints it and exits with code 1.

---

## Module System: How Files Are Organized

In JS/TS, any file can import any other file. In Rust, you build a **module tree** rooted at `main.rs` (for binaries) or `lib.rs` (for libraries).

```
src/
├── main.rs          # Root. Declares: mod cli; mod config; mod git;
├── cli/
│   ├── mod.rs       # Declares: pub mod init; pub mod clone; pub mod status;
│   ├── init.rs      # Contains: pub fn run() -> Result<()> { ... }
│   ├── clone.rs
│   └── status.rs
├── config/
│   ├── mod.rs       # Declares: pub mod schema; + pub fn load() -> Result<Config>
│   └── schema.rs    # The Config/Repo/Workspace structs
└── git/
    └── mod.rs       # Git command wrappers
```

**Key rules:**
- A directory becomes a module via `mod.rs` inside it
- `mod foo;` in `main.rs` = "include `foo.rs` or `foo/mod.rs`"
- `pub mod foo;` in `mod.rs` = "this submodule is visible to parent modules"
- `use crate::config::schema::Config;` = import from root of your crate

Think of it like: `main.rs` is the root `index.ts`, each `mod.rs` is a directory's `index.ts`, and `mod foo;` is like `export * from './foo'`.

---

## Building and Running

```bash
# Create a new project (we'll do this to start)
cargo init kimono

# Build (debug mode — fast compile, slow runtime)
cargo build

# Build + run
cargo run -- status          # Everything after -- is passed as args
cargo run -- wt feature payments backend --new

# Build release (slow compile, fast + small binary)
cargo build --release        # Binary at target/release/kimono

# Run tests
cargo test

# Add a dependency
cargo add clap --features derive
cargo add serde --features derive
cargo add serde_yaml

# Check for errors without building (faster feedback)
cargo check

# Format code (like prettier)
cargo fmt

# Lint (like eslint)
cargo clippy
```

**The `target/` directory** is like `node_modules` + `dist` combined — it holds compiled artifacts and cached dependencies. It's `.gitignore`d.

---

## What's Next

As we build kimono, I'll add more docs here:

- **02** — ownership patterns we actually use (when to `&`, when to `clone()`, when to `.to_string()`)
- **03** — error handling patterns (anyhow, context, custom errors)
- **04** — serde deep dive (how YAML becomes structs, custom deserialization)
- **05** — tera templates (how the template engine works in Rust)
- **06** — testing in Rust (unit tests, integration tests, temp directories)

Each doc will be tied to the code we're writing at that point — not abstract theory.
