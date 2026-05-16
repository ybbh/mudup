use crate::config::Config;
use anyhow::{anyhow, bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use scraper::{Html, Selector};
use std::collections::BTreeSet;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

const TAGS_URL: &str = "https://github.com/scuptio/mududb/tags";
const RELEASES_DOWNLOAD_URL_PREFIX: &str = "https://github.com/scuptio/mududb/releases/download";

#[derive(Debug)]
pub(crate) struct ChannelManifest {
    latest: String,
    releases: Vec<Release>,
}

#[derive(Debug)]
struct Release {
    version: String,
    artifacts: Vec<ReleaseArtifact>,
}

#[derive(Clone, Debug)]
pub(crate) struct ReleaseArtifact {
    pub(crate) host: String,
    pub(crate) url: String,
}

pub(crate) async fn fetch_channel_manifest(host: &str) -> Result<ChannelManifest> {
    let text = reqwest::get(TAGS_URL)
        .await
        .with_context(|| format!("download tags page {TAGS_URL}"))?
        .error_for_status()
        .with_context(|| format!("tags page request failed: {TAGS_URL}"))?
        .text()
        .await?;

    let versions: BTreeSet<String> = collect_versions(&text);
    let latest = versions
        .iter()
        .next_back()
        .cloned()
        .ok_or_else(|| anyhow!("no version like vYYYYMMDD.HHMM found in {TAGS_URL}"))?;

    let releases = versions
        .into_iter()
        .map(|version| Release {
            artifacts: vec![ReleaseArtifact {
                host: host.to_string(),
                url: format!(
                    "{RELEASES_DOWNLOAD_URL_PREFIX}/{version}/mududb-{version}-{host}.tar.gz"
                ),
            }],
            version,
        })
        .collect();

    Ok(ChannelManifest { latest, releases })
}

pub(crate) fn release_artifact_for_version(version: &str, host: &str) -> Result<ReleaseArtifact> {
    if !is_release_tag(version) {
        bail!("invalid version {version}, expected format vYYYYMMDD.HHMM");
    }
    Ok(ReleaseArtifact {
        host: host.to_string(),
        url: format!(
            "{RELEASES_DOWNLOAD_URL_PREFIX}/{version}/mududb-{version}-{host}.tar.gz"
        ),
    })
}

pub(crate) async fn fetch_sha256(url: &str) -> Result<String> {
    let text = reqwest::get(url)
        .await
        .with_context(|| format!("download checksum {url}"))?
        .error_for_status()
        .with_context(|| format!("checksum request failed: {url}"))?
        .text()
        .await?;
    text.split_whitespace()
        .next()
        .map(str::to_string)
        .ok_or_else(|| anyhow!("empty checksum response from {url}"))
}

pub(crate) fn sha256_url_for_artifact(url: &str) -> String {
    format!("{url}.sha256")
}

fn collect_versions(text: &str) -> BTreeSet<String> {
    let document = Html::parse_document(text);
    let selector = Selector::parse("a[href]").expect("valid selector");
    let mut out = BTreeSet::new();
    for node in document.select(&selector) {
        let Some(href) = node.value().attr("href") else {
            continue;
        };
        if let Some(version) = extract_version_from_href(href) {
            out.insert(version.to_string());
        }
    }
    out
}

fn extract_version_from_href(href: &str) -> Option<&str> {
    let marker = "/scuptio/mududb/releases/tag/";
    let idx = href.find(marker)?;
    let version = &href[idx + marker.len()..];
    if is_release_tag(version) {
        Some(version)
    } else {
        None
    }
}

fn is_release_tag(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 14 || bytes.first() != Some(&b'v') || bytes.get(9) != Some(&b'.') {
        return false;
    }
    bytes[1..9].iter().all(u8::is_ascii_digit) && bytes[10..14].iter().all(u8::is_ascii_digit)
}

pub(crate) fn select_channel_artifact(
    channel: &ChannelManifest,
    host: &str,
) -> Result<ReleaseArtifact> {
    let latest = channel
        .releases
        .iter()
        .find(|release| release.version == channel.latest)
        .ok_or_else(|| anyhow!("latest release {} not found in channel", channel.latest))?;

    latest
        .artifacts
        .iter()
        .find(|artifact| artifact.host == host)
        .or_else(|| latest.artifacts.first())
        .cloned()
        .ok_or_else(|| anyhow!("no artifact in release {}", latest.version))
}

pub(crate) async fn download_artifact(
    cfg: &Config,
    artifact: &ReleaseArtifact,
    version: &str,
) -> Result<PathBuf> {
    let filename = artifact
        .url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| anyhow!("artifact URL has no filename: {}", artifact.url))?;
    let archive_path = cfg.root.join("downloads").join(filename);

    let mut response = reqwest::get(&artifact.url)
        .await
        .with_context(|| format!("download artifact {}", artifact.url))?
        .error_for_status()
        .with_context(|| format!("artifact request failed: {}", artifact.url))?;

    let total = response.content_length();
    let pb = if let Some(total) = total {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} {msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, eta {eta})",
            )?
                .progress_chars("=>-"),
        );
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg} {bytes} ({bytes_per_sec})")?,
        );
        pb
    };
    pb.set_message(format!("downloading {version}"));

    let mut file = tokio::fs::File::create(&archive_path)
        .await
        .with_context(|| format!("create {}", archive_path.display()))?;
    while let Some(chunk) = response
        .chunk()
        .await
        .with_context(|| format!("stream artifact {}", artifact.url))?
    {
        file.write_all(&chunk)
            .await
            .with_context(|| format!("write {}", archive_path.display()))?;
        pb.inc(chunk.len() as u64);
    }
    file.flush()
        .await
        .with_context(|| format!("flush {}", archive_path.display()))?;
    pb.finish_with_message(format!("downloaded {version}"));

    println!("downloaded {version} to {}", archive_path.display());
    Ok(archive_path)
}

pub(crate) fn artifact_version(url: &str) -> Option<String> {
    let filename = url.rsplit('/').next()?;
    if !filename.starts_with("mududb-v") || !filename.ends_with(".tar.gz") {
        return None;
    }
    let body = filename
        .strip_prefix("mududb-")?
        .strip_suffix(".tar.gz")?;
    let mut parts = body.splitn(2, '-');
    let version = parts.next()?;
    if is_release_tag(version) {
        Some(version.to_string())
    } else {
        None
    }
}
