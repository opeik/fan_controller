//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use std::{
    env,
    fs::File,
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use data_encoding::HEXUPPER;
use sha2::{Digest, Sha256};
use url::Url;

fn main() -> Result<()> {
    // Download the chips SVD file for `probe-rs` debugging.
    download_svd_file()?;

    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .context("failed to create `memory.x` file")?
        .write_all(include_bytes!("memory.x"))
        .context("failed to write `memory.x` file")?;
    println!("cargo:rustc-link-search={}", out.display());

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

fn download_svd_file() -> Result<()> {
    let url = Url::parse(
        "https://github.com/raspberrypi/pico-sdk/raw/master/src/rp2040/hardware_regs/rp2040.svd",
    )?;
    let path = Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .to_path_buf()
        .parent()
        .context("missing parent")?
        .join("target/thumbv6m-none-eabi/release");
    download_file(
        url,
        path,
        "5A046202808F07C660ADE1A4B38AD31D10309EC1ABBAFEA4A0342FC538F626C8",
    )?;
    Ok(())
}

// fn download_pico_w_firmware() -> Result<()> {
//     let base_url = Url::parse("https://github.com/embassy-rs/embassy/raw/main/cyw43-firmware/")?;
//     let path = Path::new(&std::env::var("OUT_DIR")?).to_path_buf();
//     download_file(base_url.join("43439A0.bin")?, &path)?;
//     download_file(base_url.join("43439A0_clm.bin")?, &path)?;
//     Ok(())
// }

fn download_file<P: AsRef<Path>>(url: Url, parent_dir: P, expected_hash: &str) -> Result<()> {
    let filename = url
        .path_segments()
        .context("missing url")?
        .last()
        .context("missing filename")?;
    let path = parent_dir.as_ref().join(filename);

    if path.exists() {
        let actual_hash = file_hash(&path)?;
        if actual_hash == expected_hash {
            return Ok(());
        } else {
            return Err(anyhow!(
                "mismatched sha256 hash for `{path:?}`: found `{actual_hash}`, expected `{expected_hash}`"
            ));
        }
    }

    let contents = reqwest::blocking::get(url.clone())?.bytes()?;
    let actual_hash = hash_reader(contents.as_ref())?;
    if actual_hash != expected_hash {
        return Err(anyhow!(
            "mismatched sha256 hash for `{path:?}`: found `{actual_hash}`, expected `{expected_hash}`"
        ));
    }

    let mut file =
        File::create(parent_dir.as_ref().join(filename)).context("failed to create file")?;
    file.write_all(&contents)?;

    Ok(())
}

fn file_hash<P: AsRef<Path>>(path: P) -> Result<String> {
    let file = File::open(path)?;
    hash_reader(file)
}

fn hash_reader<R: Read>(reader: R) -> Result<String> {
    let mut reader = BufReader::new(reader);
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }

        hasher.update(&buffer[..count]);
    }

    let digest = hasher.finalize();
    Ok(HEXUPPER.encode(digest.as_ref()))
}
