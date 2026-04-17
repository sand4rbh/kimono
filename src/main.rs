mod cli;
mod config;
mod context;
mod git;
mod github;
mod ui;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Kimono — multi-repo aggregator for AI-assisted development with Claude Code
#[derive(Parser)]
#[command(name = "kimono", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new kimono workspace (creates a directory named after the workspace)
    Init {
        /// Workspace name (also used as the directory name). If omitted, prompted interactively.
        name: Option<String>,

        /// Path to an existing ofmono-style .repos.conf to bootstrap from
        #[arg(long)]
        from: Option<PathBuf>,

        /// Create a bare workspace (config only, no clone)
        #[arg(long)]
        bare: bool,
    },

    /// Add a repository to the workspace
    Add {
        /// Name for the repository
        name: String,

        /// Git remote URL
        remote: String,

        /// Default branch to track
        #[arg(long)]
        branch: Option<String>,
    },

    /// Remove a repository from the workspace
    Remove {
        /// Name of the repository to remove
        name: String,

        /// Also delete the local directory
        #[arg(long)]
        delete: bool,
    },

    /// Clone repositories defined in the workspace
    Clone {
        /// Specific repos to clone (default: all)
        repos: Vec<String>,
    },

    /// Sync repositories with their remotes
    Sync {
        /// Specific repos to sync (default: all)
        repos: Vec<String>,

        /// Only fetch, don't merge or rebase
        #[arg(long)]
        fetch_only: bool,
    },

    /// Show status of workspace repositories
    Status {
        /// Specific repos to show status for (default: all)
        repos: Vec<String>,
    },

    /// Execute a shell command across repositories
    Exec {
        /// Command to execute in each repo
        command: String,

        /// Specific repos to run in (default: all)
        repos: Vec<String>,
    },

    /// Manage branches across repositories
    Branch {
        /// Branch name
        name: String,

        /// Specific repos (default: all)
        repos: Vec<String>,

        /// Create a new branch
        #[arg(long)]
        new: bool,

        /// List branches instead of switching
        #[arg(long)]
        list: bool,

        /// Create branch from master/main
        #[arg(long)]
        from_master: bool,

        /// Don't checkout the branch after creating it
        #[arg(long)]
        no_checkout: bool,
    },

    /// Commit changes across repositories
    Commit {
        /// Specific repos to commit in (default: all)
        repos: Vec<String>,

        /// Feature name to associate the commit with
        #[arg(long)]
        feature: Option<String>,

        /// Commit message
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Create pull requests across repositories
    #[command(name = "create-pr")]
    CreatePr {
        /// Specific repos to create PRs for (default: all)
        repos: Vec<String>,

        /// Feature name to associate the PR with
        #[arg(long)]
        feature: Option<String>,

        /// Create as draft PR
        #[arg(long)]
        draft: bool,
    },

    /// Update existing pull requests across repositories
    #[command(name = "update-pr")]
    UpdatePr {
        /// Specific repos to update PRs for (default: all)
        repos: Vec<String>,

        /// Feature name to associate the PR with
        #[arg(long)]
        feature: Option<String>,
    },

    /// Show review comments on pull requests
    #[command(name = "pr-review-comments")]
    PrReviewComments {
        /// Specific repos to show comments for (default: all)
        repos: Vec<String>,

        /// Feature name to filter by
        #[arg(long)]
        feature: Option<String>,
    },

    /// Manage git worktrees across repositories
    Wt {
        #[command(subcommand)]
        command: WtCommands,
    },

    /// Manage cross-repo context for Claude Code
    Context {
        #[command(subcommand)]
        command: ContextCommands,
    },
}

#[derive(Subcommand)]
enum WtCommands {
    /// Add a worktree for a repository
    Add {
        /// Repository name
        repo: String,

        /// Branch name for the worktree
        branch: String,

        /// Create a new branch
        #[arg(long)]
        new: bool,
    },

    /// Remove a worktree from a repository
    Remove {
        /// Repository name
        repo: String,

        /// Branch name of the worktree to remove
        branch: String,
    },

    /// List worktrees for a repository
    List {
        /// Repository name (default: all repos)
        repo: Option<String>,
    },

    /// Manage feature worktrees across multiple repos
    Feature {
        /// Feature branch name
        name: String,

        /// Specific repos (default: all)
        repos: Vec<String>,

        /// Create new feature worktrees
        #[arg(long)]
        new: bool,

        /// Remove feature worktrees
        #[arg(long)]
        remove: bool,
    },

    /// Clean up merged or stale worktrees
    Clean {
        /// Only clean merged worktrees
        #[arg(long)]
        merged: bool,

        /// Show what would be cleaned without doing it
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum ContextCommands {
    /// Generate cross-repo context files for Claude Code
    Generate {
        /// Overwrite existing context files
        #[arg(long)]
        force: bool,
    },

    /// Show diff of context since last generation
    Diff,

    /// Display the current generated context
    Show,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Fast-fail if `git` isn't available on PATH. Clap handles --help and
    // --version before main runs, so this only affects actual subcommand
    // dispatch.
    git::git_available()?;

    match cli.command {
        Commands::Init { name, from, bare } => {
            cli::init::run(name.as_deref(), from.as_deref(), bare)
        }
        Commands::Add {
            name,
            remote,
            branch,
        } => cli::add::run(&name, &remote, branch.as_deref()),
        Commands::Remove { name, delete } => cli::remove::run(&name, delete),
        Commands::Clone { repos } => cli::clone::run(&repos),
        Commands::Sync { repos, fetch_only } => cli::sync::run(&repos, fetch_only),
        Commands::Status { repos } => cli::status::run(&repos),
        Commands::Exec { command, repos } => cli::exec::run(&command, &repos),
        Commands::Branch {
            name,
            repos,
            new,
            list,
            from_master,
            no_checkout,
        } => cli::branch::run(&name, &repos, new, list, from_master, no_checkout),
        Commands::Commit {
            repos,
            feature,
            message,
        } => cli::commit::run(&repos, feature.as_deref(), message.as_deref()),
        Commands::CreatePr {
            repos,
            feature,
            draft,
        } => cli::create_pr::run(&repos, feature.as_deref(), draft),
        Commands::UpdatePr { repos, feature } => cli::update_pr::run(&repos, feature.as_deref()),
        Commands::PrReviewComments { repos, feature } => {
            cli::pr_review_comments::run(&repos, feature.as_deref())
        }
        Commands::Wt { command } => match command {
            WtCommands::Add { repo, branch, new } => cli::wt::add::run(&repo, &branch, new),
            WtCommands::Remove { repo, branch } => cli::wt::remove::run(&repo, &branch),
            WtCommands::List { repo } => cli::wt::list::run(repo.as_deref()),
            WtCommands::Feature {
                name,
                repos,
                new,
                remove,
            } => cli::wt::feature::run(&name, &repos, new, remove),
            WtCommands::Clean { merged, dry_run } => cli::wt::clean::run(merged, dry_run),
        },
        Commands::Context { command } => match command {
            ContextCommands::Generate { force } => cli::context::generate::run(force),
            ContextCommands::Diff => cli::context::diff::run(),
            ContextCommands::Show => cli::context::show::run(),
        },
    }
}
