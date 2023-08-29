//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use anyhow::{Context, Result};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use url::Url;

fn main() -> Result<()> {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");

    println!("cargo:rustc-link-arg-tests=--nmagic");
    println!("cargo:rustc-link-arg-tests=-Tlink.x");
    println!("cargo:rustc-link-arg-tests=-Tlink-rp.x");
    println!("cargo:rustc-link-arg-tests=-Tdefmt.x");

    download_firmware()?;

    Ok(())
}

fn download_firmware() -> Result<()> {
    let base_url = Url::parse("https://github.com/embassy-rs/embassy/raw/main/cyw43-firmware/")?;
    let base_path = Path::new(&std::env::var("OUT_DIR")?).to_path_buf();
    download_file(base_url.join("43439A0.bin")?, &base_path)?;
    download_file(base_url.join("43439A0_clm.bin")?, &base_path)?;
    Ok(())
}

fn download_file<P: AsRef<Path>>(url: Url, parent: P) -> Result<()> {
    let contents = reqwest::blocking::get(url.clone())?.bytes()?;
    let filename = url
        .path_segments()
        .context("missing url")?
        .last()
        .context("missing filename")?;
    let mut file = File::create(parent.as_ref().join(filename))?;
    file.write_all(&contents)?;
    Ok(())
}
