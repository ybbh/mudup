use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use crate::config::DEFAULT_CHANNEL;

#[derive(Parser, Debug)]
#[command(name = "mudup")]
#[command(version)]
#[command(about = "MuduDB and its toolchain installer and version manager")]
pub(crate) struct Cli {
    #[arg(long, global = true, help = "Override the mudup root directory.")]
    pub(crate) root: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        default_value = DEFAULT_CHANNEL,
        help = "Release channel (reserved for compatibility)."
    )]
    pub(crate) channel: String,
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Install a version such as v20260514.1144; if omitted, install the latest tag.
    Install(InstallArgs),
    /// Update MuduDB and its toolchain to the latest release.
    Update,
    #[command(name = "self")]
    SelfCmd(SelfArgs),
    /// List installed releases.
    List,
    /// Remove an installed version.
    Uninstall(UninstallArgs),
}

#[derive(Args, Debug)]
pub(crate) struct InstallArgs {
    #[arg(help = "Version to install; omit to install the latest tag.")]
    pub(crate) version: Option<String>,
}

#[derive(Args, Debug)]
pub(crate) struct SelfArgs {
    #[command(subcommand)]
    pub(crate) command: SelfCommands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum SelfCommands {
    /// Update mudup itself to the latest release.
    Update,
}

#[derive(Args, Debug)]
pub(crate) struct UninstallArgs {
    pub(crate) version: String,
}
