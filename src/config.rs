use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub(crate) const DEFAULT_CHANNEL: &str = "latest";
pub(crate) const TOOL_BINARIES: &[&str] = &["mudud", "mcli", "mpk", "mgen", "mtp"];

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) root: PathBuf,
}

impl Config {
    pub(crate) fn new(root: Option<PathBuf>, _channel: String) -> Result<Self> {
        Ok(Self {
            root: resolve_root(root)?,
        })
    }
}

fn resolve_root(root: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(root) = root {
        return Ok(root);
    }
    if let Ok(root) = std::env::var("MUDUP_HOME") {
        return Ok(PathBuf::from(root));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home).join(".mududb"))
}

pub(crate) fn host_triple() -> Result<String> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "linux") => Ok("x86_64-unknown-linux-gnu".to_string()),
        (arch, os) => anyhow::bail!("unsupported host {arch}-{os}"),
    }
}

pub(crate) fn mududb_cfg_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home)
        .join(".mududb")
        .join("mududb_cfg.toml"))
}
