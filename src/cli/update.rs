use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;

const REPO: &str = "rijuma/ctxhelpr";
const SKILL_CONTENT: &str = include_str!("../assets/skill.md");
const INDEX_COMMAND_CONTENT: &str = include_str!("../assets/index_command.md");

pub fn run() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    let platform = detect_platform()?;

    println!("ctxhelpr v{current_version} ({platform})\n");

    print!("  Checking for updates... ");
    let latest_version = fetch_latest_version()?;
    println!("v{latest_version}");

    if current_version == latest_version {
        println!("\n  Already up to date!");
        refresh_skill_files();
        return Ok(());
    }

    println!("\n  Updating v{current_version} -> v{latest_version}...\n");

    let tmpdir = std::env::temp_dir().join(format!("ctxhelpr-update-{latest_version}"));
    fs::create_dir_all(&tmpdir)?;

    let asset = format!("ctxhelpr-{latest_version}-{platform}.tar.gz");
    let base_url = format!("https://github.com/{REPO}/releases/download/v{latest_version}");

    // Download tarball
    print!("  Downloading {asset}... ");
    let tarball_path = tmpdir.join(&asset);
    download_file(&format!("{base_url}/{asset}"), &tarball_path)?;
    println!("done");

    // Verify checksum
    let checksum_path = tmpdir.join(format!("{asset}.sha256"));
    if download_file(&format!("{base_url}/{asset}.sha256"), &checksum_path).is_ok() {
        print!("  Verifying checksum... ");
        verify_checksum(&tarball_path, &checksum_path)?;
        println!("done");
    }

    // Extract
    print!("  Extracting... ");
    let status = Command::new("tar")
        .args(["xzf", tarball_path.to_str().unwrap(), "-C"])
        .arg(&tmpdir)
        .status()
        .context("failed to run tar")?;
    if !status.success() {
        bail!("tar extraction failed");
    }
    println!("done");

    // Replace binary
    print!("  Replacing binary... ");
    let current_exe = std::env::current_exe().context("cannot determine current binary path")?;
    let new_binary = tmpdir.join("ctxhelpr");
    replace_binary(&new_binary, &current_exe)?;
    println!("done");

    // Cleanup
    let _ = fs::remove_dir_all(&tmpdir);

    // Refresh skill files
    refresh_skill_files();

    println!("\n  Updated to v{latest_version}!");
    println!(
        "\n  Consider re-indexing your repositories to take advantage of any indexing improvements."
    );

    Ok(())
}

fn detect_platform() -> Result<String> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        other => bail!("unsupported OS: {other}"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => bail!("unsupported architecture: {other}"),
    };
    Ok(format!("{os}-{arch}"))
}

fn fetch_latest_version() -> Result<String> {
    let output = http_get(&format!(
        "https://api.github.com/repos/{REPO}/releases/latest"
    ))?;
    let response = String::from_utf8(output).context("invalid UTF-8 in API response")?;

    // Parse tag_name from JSON (avoid adding serde_json dependency for this)
    // Format: "tag_name": "v1.2.3"
    let tag = response
        .lines()
        .find(|line| line.contains("\"tag_name\""))
        .and_then(|line| {
            let start = line.find('"').and_then(|i| {
                line[i + 1..].find('"').and_then(|j| {
                    line[i + 1 + j + 1..]
                        .find('"')
                        .map(|k| i + 1 + j + 1 + k + 1)
                })
            })?;
            let end = line[start..].find('"').map(|i| i + start)?;
            Some(line[start..end].to_string())
        })
        .context("could not parse tag_name from GitHub API response")?;

    // Strip leading 'v' if present
    Ok(tag.strip_prefix('v').unwrap_or(&tag).to_string())
}

fn http_get(url: &str) -> Result<Vec<u8>> {
    // Try curl first, then wget
    let output = Command::new("curl")
        .args(["--proto", "=https", "--tlsv1.2", "-sSfL", url])
        .output();

    match output {
        Ok(o) if o.status.success() => return Ok(o.stdout),
        _ => {}
    }

    let output = Command::new("wget")
        .args(["--https-only", "--quiet", "-O-", url])
        .output()
        .context("neither curl nor wget is available")?;

    if !output.status.success() {
        bail!("failed to fetch {url}");
    }
    Ok(output.stdout)
}

fn download_file(url: &str, dest: &Path) -> Result<()> {
    let dest_str = dest.to_str().context("invalid path")?;

    // Try curl first, then wget
    let status = Command::new("curl")
        .args([
            "--proto",
            "=https",
            "--tlsv1.2",
            "-sSfL",
            "-o",
            dest_str,
            url,
        ])
        .status();

    match status {
        Ok(s) if s.success() => return Ok(()),
        _ => {}
    }

    let status = Command::new("wget")
        .args(["--https-only", "--quiet", "-O", dest_str, url])
        .status()
        .context("neither curl nor wget is available")?;

    if !status.success() {
        bail!("failed to download {url}");
    }
    Ok(())
}

fn verify_checksum(tarball: &Path, checksum_file: &Path) -> Result<()> {
    let checksum_content = fs::read_to_string(checksum_file)?;
    let expected = checksum_content
        .split_whitespace()
        .next()
        .context("empty checksum file")?;

    let tarball_str = tarball.to_str().context("invalid path")?;

    // Try sha256sum first, then shasum
    let actual = Command::new("sha256sum")
        .arg(tarball_str)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .or_else(|| {
            Command::new("shasum")
                .args(["-a", "256", tarball_str])
                .output()
                .ok()
                .filter(|o| o.status.success())
        })
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .and_then(|s| s.split_whitespace().next().map(|h| h.to_string()))
        })
        .context("no sha256sum or shasum available")?;

    if expected != actual {
        bail!("checksum mismatch (expected {expected}, got {actual})");
    }
    Ok(())
}

fn replace_binary(new_binary: &Path, current_exe: &Path) -> Result<()> {
    let parent = current_exe
        .parent()
        .context("cannot determine binary directory")?;
    let tmp_dest = parent.join(".ctxhelpr-update-tmp");

    // Copy new binary to temp file in same directory (for atomic rename)
    fs::copy(new_binary, &tmp_dest).context("failed to copy new binary")?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_dest, fs::Permissions::from_mode(0o755))?;
    }

    // Atomic rename
    fs::rename(&tmp_dest, current_exe).context("failed to replace binary")?;

    Ok(())
}

fn refresh_skill_files() {
    let paths_to_check: Vec<std::path::PathBuf> = [
        dirs::home_dir().map(|h| h.join(".claude")),
        std::env::current_dir().ok().map(|d| d.join(".claude")),
    ]
    .into_iter()
    .flatten()
    .collect();

    let mut refreshed = false;
    for base in &paths_to_check {
        let skill_path = base.join("skills").join("ctxhelpr").join("SKILL.md");
        if skill_path.exists() {
            if let Err(e) = fs::write(&skill_path, SKILL_CONTENT) {
                eprintln!("  Warning: could not refresh skill file: {e}");
            } else {
                refreshed = true;
            }
        }

        let cmd_path = base.join("commands").join("index.md");
        if cmd_path.exists() {
            if let Err(e) = fs::write(&cmd_path, INDEX_COMMAND_CONTENT) {
                eprintln!("  Warning: could not refresh command file: {e}");
            } else {
                refreshed = true;
            }
        }
    }

    if refreshed {
        println!("  Refreshed skill and command files.");
    }
}
