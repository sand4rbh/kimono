// Plugin module is being built up across Tasks 3-6. Suppress dead-code
// warnings for foundation items not yet wired into the CLI; the allow is
// removed once all submodules are filled in.
#![allow(dead_code)]

pub mod claude_shell;
pub mod install;
pub mod registry;
