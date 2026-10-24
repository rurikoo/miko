use std::path::Path;

use anyhow::{Context, Result, anyhow};
use gpapi::Gpapi;
use gpapi::error::ErrorKind as GpapiErrorKind;
use tokio::runtime::Runtime;

use crate::arch::Arch;
use crate::auth::fetch_public_dispenser_auth;
use crate::device::{DeviceSource, ResolvedDevice, resolve_device};
use crate::error::map_gpapi_error;

/// Downloads package into specific directory
pub fn download_apk(
    package: &str,
    source: DeviceSource,
    arch: Option<Arch>,
    out_dir: &Path,
) -> Result<()> {
    let device = resolve_device(source, arch)?;

    let result = fetch_public_dispenser_auth()
        .and_then(|(email, token)| run_download(&device, email, token, package, out_dir));

    if let Some(tmp) = &device.temp {
        let _ = std::fs::remove_file(tmp);
    }

    result
}

fn run_download(
    device: &ResolvedDevice,
    email: String,
    token: String,
    package: &str,
    out_dir: &Path,
) -> Result<()> {
    let props_path: Option<String> = device
        .props_path
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned());

    let rt = Runtime::new().context("failed to create tokio runtime")?;
    rt.block_on(async {
        let mut api = match &props_path {
            Some(p) => Gpapi::from_device_properties_file(&device.profile, &email, p),
            None => Gpapi::new(&device.profile, &email),
        };
        api.set_auth_token(token);

        match api.login().await {
            Err(e) if matches!(e.kind(), GpapiErrorKind::TermsOfService) => {
                api.accept_tos()
                    .await
                    .map_err(|e| anyhow!("{e}"))
                    .context("failed to accept the TOS of gplay")?;
            }
            Err(e) => {
                return Err(map_gpapi_error(e)).context("failed to log in to gplay");
            }
            Ok(()) => {}
        }

        // split_if_available = true so config.<abi> splits into multiple apks
        // lowk good for me
        api.download(package, None, true, false, false, out_dir, None)
            .await
            .map_err(map_gpapi_error)?;

        Ok::<(), anyhow::Error>(())
    })
}
