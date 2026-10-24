use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, bail};

use crate::arch::Arch;

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn temp_path() -> PathBuf {
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("miko-device-{}-{n}.properties", std::process::id()))
}

/// Reads the device.properties, patches the target section's Platforms line, writes a temp copy
pub(crate) fn patched_properties(src: &Path, section: &str, arch: Arch) -> Result<PathBuf> {
    let text = std::fs::read_to_string(src)
        .with_context(|| format!("failed to read device properties {}", src.display()))?;
    let patched = patch_platforms(&text, section, arch)?;

    let tmp = temp_path();
    std::fs::write(&tmp, patched).context("failed to write temporary device properties")?;
    Ok(tmp)
}

/// Returns the text with only the target section's Platforms line replaced
pub(crate) fn patch_platforms(text: &str, section: &str, arch: Arch) -> Result<String> {
    let header = format!("[{section}]");
    let mut in_section = false;
    let mut found_section = false;
    let mut found_platforms = false;
    let mut out = String::with_capacity(text.len());

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed == header;
            found_section |= in_section;
        }
        if in_section && trimmed.starts_with("Platforms=") {
            out.push_str("Platforms=");
            out.push_str(arch.platforms());
            found_platforms = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }

    if !found_section {
        bail!("section [{section}] not found in device properties");
    }
    if !found_platforms {
        bail!("section [{section}] has no Platforms= line to set the arch on");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_changes_only_target_section() {
        let text = "[a]\nPlatforms=arm64-v8a\nBuild.ID=1\n[b]\nPlatforms=arm64-v8a\nBuild.ID=2\n";
        let out = patch_platforms(text, "b", Arch::X86_64).unwrap();

        assert!(out.contains("[a]\nPlatforms=arm64-v8a\n"));
        assert!(out.contains("[b]\nPlatforms=x86_64,x86\n"));
        assert!(out.contains("Build.ID=2"));
        assert!(out.contains("Build.ID=1"));
    }

    #[test]
    fn patch_keeps_other_lines_intact() {
        let text = "[dev]\nBuild.ID=abc\nPlatforms=arm64-v8a\nTouchScreen=3\n";
        let out = patch_platforms(text, "dev", Arch::ArmV7a).unwrap();
        assert_eq!(
            out,
            "[dev]\nBuild.ID=abc\nPlatforms=armeabi-v7a,armeabi\nTouchScreen=3\n"
        );
    }

    #[test]
    fn patch_errors_when_section_missing() {
        let err = patch_platforms("[a]\nPlatforms=x\n", "nope", Arch::X86_64)
            .unwrap_err()
            .to_string();
        assert!(err.contains("section [nope] not found"));
    }

    #[test]
    fn patch_errors_when_no_platforms_line() {
        let err = patch_platforms("[a]\nBuild.ID=1\n", "a", Arch::X86_64)
            .unwrap_err()
            .to_string();
        assert!(err.contains("has no Platforms="));
    }
}
