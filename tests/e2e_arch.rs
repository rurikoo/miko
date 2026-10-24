//! run with:
//!   MIKO_E2E_PACKAGE=com.example.app cargo test --test e2e_arch -- --ignored --nocapture
//! optional env vars:
//!   MIKO_E2E_DEVICE_FILE  device.properties path (enables the per-arch run)
//!   MIKO_E2E_SECTION      section to use in that file (required with MIKO_E2E_DEVICE_FILE)
//!   MIKO_E2E_PROFILE      builtin profile name when no file is given (default: px_9a)
//!   MIKO_E2E_EXPECT_ABI   abi to assert on in builtin mode, e.g. x86_64

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use miko::{Arch, DeviceSource, download_apk};

const ARCHES: &[(Arch, &str)] = &[
    (Arch::X86_64, "x86_64"),
    (Arch::Arm64V8a, "arm64-v8a"),
    (Arch::ArmV7a, "armeabi-v7a"),
];

fn env_opt(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}

#[test]
#[ignore]
fn downloads_app() {
    let package = env_opt("MIKO_E2E_PACKAGE").expect("set MIKO_E2E_PACKAGE to run the e2e test");
    let root = std::env::temp_dir().join(format!("miko-e2e-{}", std::process::id()));
    let mut failures = Vec::new();

    match env_opt("MIKO_E2E_DEVICE_FILE") {
        Some(file) => {
            let section = env_opt("MIKO_E2E_SECTION")
                .expect("set MIKO_E2E_SECTION when using MIKO_E2E_DEVICE_FILE");
            let file = PathBuf::from(file);

            for &(arch, abi) in ARCHES {
                let out = root.join(abi);
                let source = DeviceSource::File {
                    path: &file,
                    section: &section,
                };
                if let Err(msg) = run_case(&package, source, Some(arch), &out, Some(abi)) {
                    failures.push(format!("{abi}: {msg}"));
                }
            }
        }
        None => {
            let profile = env_opt("MIKO_E2E_PROFILE").unwrap_or_else(|| "px_9a".to_string());
            let expect = env_opt("MIKO_E2E_EXPECT_ABI");
            let out = root.join(&profile);
            let source = DeviceSource::Builtin(&profile);

            if let Err(msg) = run_case(&package, source, None, &out, expect.as_deref()) {
                failures.push(format!("builtin {profile}: {msg}"));
            }
        }
    }

    if failures.is_empty() {
        let _ = fs::remove_dir_all(&root);
    } else {
        eprintln!("downloads kept for inspection in {}", root.display());
    }

    assert!(
        failures.is_empty(),
        "e2e failures:\n{}",
        failures.join("\n")
    );
}

fn run_case(
    package: &str,
    source: DeviceSource,
    arch: Option<Arch>,
    out: &Path,
    expect_abi: Option<&str>,
) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|e| format!("failed to create output dir: {e}"))?;

    download_apk(package, source, arch, out).map_err(|e| format!("download failed: {e:#}"))?;

    let abis = scan_abis(out)?;
    eprintln!("{}: native abis found: {:?}", out.display(), abis);

    if abis.is_empty() {
        return Ok(());
    }

    match expect_abi {
        Some(want) if !abis.contains(want) => Err(format!(
            "expected lib/{want}/ but the apk only has {abis:?}"
        )),
        _ => Ok(()),
    }
}

fn scan_abis(out: &Path) -> Result<BTreeSet<String>, String> {
    let apks = find_apks(out).map_err(|e| format!("failed to scan output dir: {e}"))?;
    if apks.is_empty() {
        return Err("no .apk files were written".to_string());
    }

    let mut abis = BTreeSet::new();
    for apk in &apks {
        let file = File::open(apk).map_err(|e| format!("{}: {e}", apk.display()))?;
        let zip = zip::ZipArchive::new(file)
            .map_err(|e| format!("{} is not a valid zip: {e}", apk.display()))?;

        for name in zip.file_names() {
            let name = name.map_err(|e| format!("{}: bad entry name: {e}", apk.display()))?;
            if let Some(rest) = name.strip_prefix("lib/")
                && let Some((lib_abi, _)) = rest.split_once('/')
            {
                abis.insert(lib_abi.to_string());
            }
        }
    }
    Ok(abis)
}

fn find_apks(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(find_apks(&path)?);
        } else if path.extension().is_some_and(|ext| ext == "apk") {
            found.push(path);
        }
    }
    Ok(found)
}
