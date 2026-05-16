use anyhow::{anyhow, bail, Context, Result};
use std::env::current_exe;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;

use crate::checksum::verify_sha256;
use crate::config::host_triple;
use crate::remote::fetch_sha256;
use crate::util::run_command;

const LATEST_RELEASE_DOWNLOAD_URL: &str =
    "https://github.com/scuptio/mududb/releases/latest/download";

pub(crate) async fn self_update() -> Result<()> {
    let target = host_triple()?;
    let archive_name = format!("mudup-{target}.tar.gz");
    let archive_url = format!("{LATEST_RELEASE_DOWNLOAD_URL}/{archive_name}");
    let checksum_url = format!("{archive_url}.sha256");

    let tmp_root = std::env::temp_dir().join(format!("mudup-self-update-{}", nanos_since_epoch()?));
    fs::create_dir_all(&tmp_root)?;
    let archive_path = tmp_root.join(&archive_name);

    download_file(&archive_url, &archive_path).await?;
    let checksum = fetch_sha256(&checksum_url).await?;
    verify_sha256(&archive_path, &checksum)?;

    run_command(
        Command::new("tar")
            .arg("-xzf")
            .arg(&archive_path)
            .arg("-C")
            .arg(&tmp_root),
    )?;

    let new_mudup = find_mudup_binary(&tmp_root)?
        .ok_or_else(|| anyhow!("mudup binary not found in {}", archive_name))?;
    let current = current_exe().context("resolve current mudup executable path")?;
    let next = current.with_extension("new");
    let backup = current.with_extension("bak");

    fs::copy(&new_mudup, &next).with_context(|| {
        format!(
            "copy downloaded mudup from {} to {}",
            new_mudup.display(),
            next.display()
        )
    })?;

    let perms = fs::metadata(&current)
        .with_context(|| format!("read metadata from {}", current.display()))?
        .permissions();
    fs::set_permissions(&next, perms)
        .with_context(|| format!("set permissions on {}", next.display()))?;

    let _ = fs::remove_file(&backup);
    fs::rename(&current, &backup).with_context(|| {
        format!(
            "rename current mudup {} to backup {}",
            current.display(),
            backup.display()
        )
    })?;

    if let Err(err) = fs::rename(&next, &current) {
        let _ = fs::rename(&backup, &current);
        let _ = fs::remove_file(&next);
        bail!(
            "replace mudup binary failed ({} -> {}): {err}",
            next.display(),
            current.display()
        );
    }

    let _ = fs::remove_file(&backup);
    let _ = fs::remove_dir_all(&tmp_root);

    println!("mudup self updated successfully");
    Ok(())
}

async fn download_file(url: &str, path: &Path) -> Result<()> {
    let mut response = reqwest::get(url)
        .await
        .with_context(|| format!("download {}", url))?
        .error_for_status()
        .with_context(|| format!("request failed: {}", url))?;

    let mut file = tokio::fs::File::create(path)
        .await
        .with_context(|| format!("create {}", path.display()))?;

    while let Some(chunk) = response
        .chunk()
        .await
        .with_context(|| format!("stream {}", url))?
    {
        file.write_all(&chunk)
            .await
            .with_context(|| format!("write {}", path.display()))?;
    }
    file.flush()
        .await
        .with_context(|| format!("flush {}", path.display()))?;
    Ok(())
}

fn find_mudup_binary(root: &Path) -> Result<Option<PathBuf>> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(entry_path);
                continue;
            }
            if entry_path.file_name().and_then(|v| v.to_str()) == Some("mudup") {
                return Ok(Some(entry_path));
            }
        }
    }
    Ok(None)
}

fn nanos_since_epoch() -> Result<u128> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow!("system time error: {e}"))?
        .as_nanos())
}
