//! Shared by the test binaries that run the real `packie` engine: where it is and the host
//! options to run scenarios with.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use packie_sim::resources::Capabilities;
use packie_sim::runner::{HostOptions, locate_engine};

pub(crate) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The `packie` binary: found next to the test executable, or `PACKIE_BIN`, or built with
/// `cargo build -p packie` into the usual target directory (`CARGO_TARGET_DIR` respected).
pub(crate) fn engine_binary() -> PathBuf {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        if let Ok(path) = locate_engine(None) {
            return path;
        }
        let status = Command::new("cargo")
            .args(["build", "-p", "packie"])
            .current_dir(workspace_root())
            .status()
            .expect("run cargo build -p packie");
        assert!(status.success(), "cargo build -p packie failed");
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map_or_else(|| workspace_root().join("target"), PathBuf::from);
        let name = if cfg!(windows) {
            "packie.exe"
        } else {
            "packie"
        };
        let bin = target.join("debug").join(name);
        assert!(bin.is_file(), "built engine not found at {}", bin.display());
        bin
    })
    .clone()
}

pub(crate) fn host(capabilities: Capabilities) -> HostOptions {
    HostOptions {
        engine_bin: engine_binary(),
        keep_state: false,
        verbose: false,
        capabilities,
        on_step: None,
    }
}

pub(crate) async fn real_host() -> HostOptions {
    host(Capabilities::detect(&engine_binary(), false).await)
}
