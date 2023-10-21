//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use std::{env, path::PathBuf};

use anyhow::{Context, Result};

fn main() -> Result<()> {
    // Put `memory.x` linker script in our output directory and ensure it's in the linker search path.
    let rp_pico_linker_script_path =
        PathBuf::from(env::var_os("RP_PICO_LINKER_SCRIPT").context("unset linker script path")?);

    println!(
        "cargo:rustc-link-search={}",
        rp_pico_linker_script_path.display()
    );

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rustc-link-arg=--nmagic");
    println!("cargo:rustc-link-arg=-Tlink.x");
    println!("cargo:rustc-link-arg=-Tlink-rp.x");
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    Ok(())
}
