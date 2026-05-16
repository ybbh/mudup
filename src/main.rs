mod archive;
mod checksum;
mod cli;
mod config;
mod libs;
mod remote;
mod self_update;
mod toolchain;
mod util;

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::fs;
use std::process::Command;

use crate::archive::{extract_toolchain, validate_toolchain};
use crate::checksum::verify_sha256;
use crate::cli::{Cli, Commands, SelfCommands};
use crate::config::{host_triple, mududb_cfg_path, Config};
use crate::libs::check_system_libraries;
use crate::remote::{
    artifact_version, download_artifact, fetch_channel_manifest, fetch_sha256,
    release_artifact_for_version, select_channel_artifact, sha256_url_for_artifact,
};
use crate::self_update::self_update;
use crate::toolchain::{
    activate_toolchain, ensure_layout, list_releases, print_path_hint, refresh_proxies, uninstall,
};
use crate::util::run_command;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let cfg = Config::new(cli.root, cli.channel)?;

    match cli.command {
        Commands::Install(args) => {
            let requested = args.version.as_deref().unwrap_or("latest");
            install(&cfg, requested).await
        }
        Commands::Update => {
            let result = install(&cfg, "latest").await;
            if result.is_ok() {
                println!("to update mudup itself, run: mudup self update");
            }
            result
        }
        Commands::SelfCmd(args) => match args.command {
            SelfCommands::Update => self_update().await,
        },
        Commands::List => list_releases(&cfg),
        Commands::Uninstall(args) => uninstall(&cfg, &args.version),
    }
}

async fn install(cfg: &Config, requested: &str) -> Result<()> {
    ensure_layout(&cfg.root)?;

    let host = host_triple()?;
    let artifact = if requested == "latest" {
        let channel = fetch_channel_manifest(&host).await?;
        select_channel_artifact(&channel, &host)?
    } else {
        release_artifact_for_version(requested, &host)?
    };

    let version = artifact_version(&artifact.url).unwrap_or_else(|| requested.to_string());
    let sha256_url = sha256_url_for_artifact(&artifact.url);
    let sha256 = fetch_sha256(&sha256_url).await?;
    let archive_path = download_artifact(cfg, &artifact, &version).await?;
    verify_sha256(&archive_path, &sha256)?;
    check_system_libraries(cfg, &archive_path, &version)?;

    let install_dir = extract_toolchain(cfg, &archive_path, &version)?;
    install_prerequisites(&install_dir, &host)?;
    validate_toolchain(&install_dir)?;
    activate_toolchain(cfg, &version)?;
    refresh_proxies(cfg)?;
    ensure_default_mududb_cfg()?;

    println!("installed {version} for {}", artifact.host);
    print_path_hint(cfg);
    Ok(())
}

fn install_prerequisites(install_dir: &std::path::Path, host: &str) -> Result<()> {
    let script_path = {
        let script_dir_path = install_dir.join("script").join("prerequisite.sh");
        if script_dir_path.is_file() {
            script_dir_path
        } else {
            let legacy_path = install_dir.join("build-release").join("prerequisite.sh");
            if legacy_path.is_file() {
                legacy_path
            } else {
                bail!(
                    "prerequisite script not found in extracted toolchain: {} or {}",
                    install_dir.join("script").join("prerequisite.sh").display(),
                    install_dir.join("build-release").join("prerequisite.sh").display()
                );
            }
        }
    };
    run_command(
        Command::new("sh")
            .arg(&script_path)
            .arg("--install-dir")
            .arg(install_dir)
            .arg("--target")
            .arg(host),
    )
    .with_context(|| format!("install prerequisites via {}", script_path.display()))?;
    Ok(())
}

fn ensure_default_mududb_cfg() -> Result<()> {
    let cfg_path = mududb_cfg_path()?;
    if cfg_path.exists() {
        return Ok(());
    }
    if let Some(parent) = cfg_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        cfg_path,
        r#"mpk_path = "/tmp"
data_path = "/tmp"
listen_ip = "127.0.0.1"
http_listen_port = 8300
http_worker_threads = 1
pg_listen_port = 5432
enable_async = true
server_mode = 0
tcp_listen_port = 9527
io_uring_worker_threads = 0
io_uring_ring_entries = 1024
io_uring_accept_multishot = true
io_uring_recv_multishot = true
io_uring_enable_fixed_buffers = false
io_uring_enable_fixed_files = false
routing_mode = 0
io_uring_log_chunk_size = 67108864
"#,
    )?;
    Ok(())
}
