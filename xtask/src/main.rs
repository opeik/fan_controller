use std::{env, path::PathBuf};

use anyhow::Error;
use xshell::cmd;

type Result<T> = std::result::Result<T, Error>;

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let args = args.iter().map(|s| &**s).collect::<Vec<_>>();

    match &args[..] {
        ["run"] => run(),
        ["build"] => build_all(),
        ["build", "host"] => build_host(),
        ["build", "target"] => build_target(),
        ["test"] => test_all(),
        ["test", "host"] => test_host(),
        ["test", "target"] => test_target(),
        ["clean"] => clean_all(),
        ["clean", "host"] => clean_host(),
        ["clean", "target"] => clean_target(),
        _ => {
            println!("USAGE cargo xtask <build|test|clean> [host|target]");
            Ok(())
        }
    }
}

fn run() -> Result<()> {
    let _p = xshell::pushd(root_dir().join("cross"))?;
    cmd!("cargo run --release").run()?;
    Ok(())
}

fn build_all() -> Result<()> {
    build_host()?;
    build_target()?;
    Ok(())
}

fn build_host() -> Result<()> {
    let _p = xshell::pushd(root_dir())?;
    cmd!("cargo build --workspace").run()?;
    Ok(())
}

fn build_target() -> Result<()> {
    let _p = xshell::pushd(root_dir().join("cross"))?;
    cmd!("cargo build --release --package app").run()?;
    Ok(())
}

fn test_all() -> Result<()> {
    test_host()?;
    test_target()?;
    Ok(())
}

fn test_host() -> Result<()> {
    let _p = xshell::pushd(root_dir())?;
    cmd!("cargo test --workspace").run()?;
    Ok(())
}

fn test_target() -> Result<()> {
    let _p = xshell::pushd(root_dir().join("cross"))?;
    cmd!("cargo test --package self_tests").run()?;
    Ok(())
}

fn clean_all() -> Result<()> {
    clean_host()?;
    clean_target()?;
    Ok(())
}

fn clean_host() -> Result<()> {
    let _p = xshell::pushd(root_dir())?;
    cmd!("cargo clean").run()?;
    Ok(())
}

fn clean_target() -> Result<()> {
    let _p = xshell::pushd(root_dir().join("cross"))?;
    cmd!("cargo clean").run()?;
    Ok(())
}

fn root_dir() -> PathBuf {
    let mut xtask_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    xtask_dir.pop();
    xtask_dir
}
