use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::arch::Arch;
use crate::properties::patched_properties;

/// Where the device profile comes from
pub enum DeviceSource<'a> {
    /// A profile name compiled into gpapi, e.g. "px_9a"
    Builtin(&'a str),
    /// A device.properties file and the section to use
    File { path: &'a Path, section: &'a str },
}

/// What gpapi needs to build its client
#[derive(Debug)]
pub(crate) struct ResolvedDevice {
    pub(crate) profile: String,
    pub(crate) props_path: Option<PathBuf>,
    pub(crate) temp: Option<PathBuf>,
}

pub(crate) fn resolve_device(source: DeviceSource, arch: Option<Arch>) -> Result<ResolvedDevice> {
    match source {
        DeviceSource::Builtin(name) => {
            if arch.is_some() {
                bail!(
                    "arch can only be applied to a device file, built-in profiles have fixed abis"
                );
            }
            Ok(ResolvedDevice {
                profile: name.to_string(),
                props_path: None,
                temp: None,
            })
        }
        DeviceSource::File { path, section } => match arch {
            None => Ok(ResolvedDevice {
                profile: section.to_string(),
                props_path: Some(path.to_path_buf()),
                temp: None,
            }),
            Some(arch) => {
                let tmp = patched_properties(path, section, arch)?;
                Ok(ResolvedDevice {
                    profile: section.to_string(),
                    props_path: Some(tmp.clone()),
                    temp: Some(tmp),
                })
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::temp_path;
    use std::fs;

    fn write_temp_src(contents: &str) -> PathBuf {
        let p = temp_path();
        fs::write(&p, contents).unwrap();
        p
    }

    #[test]
    fn builtin_without_arch_resolves() {
        let d = resolve_device(DeviceSource::Builtin("px_9a"), None).unwrap();
        assert_eq!(d.profile, "px_9a");
        assert!(d.props_path.is_none());
        assert!(d.temp.is_none());
    }

    #[test]
    fn builtin_with_arch_is_rejected_before_network() {
        let err = resolve_device(DeviceSource::Builtin("px_9a"), Some(Arch::X86_64))
            .unwrap_err()
            .to_string();
        assert!(err.contains("arch can only be applied to a device file"));
    }

    #[test]
    fn file_without_arch_uses_path_directly() {
        let path = Path::new("/some/device.properties");
        let d = resolve_device(
            DeviceSource::File {
                path,
                section: "dev",
            },
            None,
        )
        .unwrap();
        assert_eq!(d.profile, "dev");
        assert_eq!(d.props_path.as_deref(), Some(path));
        assert!(d.temp.is_none());
    }

    #[test]
    fn file_with_arch_writes_patched_temp_copy() {
        let src = write_temp_src("[dev]\nPlatforms=arm64-v8a\nBuild.ID=x\n");
        let d = resolve_device(
            DeviceSource::File {
                path: &src,
                section: "dev",
            },
            Some(Arch::X86_64),
        )
        .unwrap();

        let tmp = d.props_path.clone().unwrap();
        assert_eq!(d.temp.as_deref(), Some(tmp.as_path()));
        let written = fs::read_to_string(&tmp).unwrap();
        assert!(written.contains("Platforms=x86_64,x86"));
        assert!(!written.contains("arm64-v8a"));

        let original = fs::read_to_string(&src).unwrap();
        assert!(original.contains("Platforms=arm64-v8a"));

        fs::remove_file(tmp).unwrap();
        fs::remove_file(src).unwrap();
    }

    #[test]
    fn two_patched_copies_get_different_paths() {
        let src = write_temp_src("[dev]\nPlatforms=arm64-v8a\n");
        let a = resolve_device(
            DeviceSource::File {
                path: &src,
                section: "dev",
            },
            Some(Arch::X86_64),
        )
        .unwrap();
        let b = resolve_device(
            DeviceSource::File {
                path: &src,
                section: "dev",
            },
            Some(Arch::X86_64),
        )
        .unwrap();
        assert_ne!(a.temp, b.temp);

        for t in [a.temp, b.temp].into_iter().flatten() {
            fs::remove_file(t).unwrap();
        }
        fs::remove_file(src).unwrap();
    }
}
