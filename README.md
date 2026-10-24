<h1 vertical-align="baseline">
  <img src=".github/assets/miko.webp" height="42" align="bottom" /> Miko
</h1>

A library for using Google Play via Aurora Store's dispenser

## Getting Started

Add Miko to your `Cargo.toml`:

```toml
[dependencies]
miko = { git = "https://github.com/rurikoo/miko" }
```

If you use the examples below as written, also add `anyhow` to your project.

### Downloading an APK

```rust
use std::path::Path;
use miko::{DeviceSource, download_apk};

fn main() -> anyhow::Result<()> {
    // a device profile built into gpapi
    download_apk(
        "com.example.app",
        DeviceSource::Builtin("px_9a"),
        None,
        Path::new("/tmp/out"),
    )?;
    Ok(())
}
```

### Choosing an architecture

Available arches:

- `Arch::X86_64`
- `Arch::Arm64V8a`
- `Arch::ArmV7a`

Built-in profiles have fixed ABIs, so `arch` only works with a device file. Pass a `device.properties` section and the arch you want:

```rust
use miko::{Arch, DeviceSource, download_apk};

download_apk(
    "com.example.app",
    DeviceSource::File {
        path: Path::new("/path/to/device.properties"),
        section: "px_9a",
    },
    Some(Arch::X86_64),
    Path::new("/tmp/out"),
)?;
```

## License

Miko is licensed under the MIT License. See [LICENSE](LICENSE).
