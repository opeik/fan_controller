use std::{env, path::PathBuf};

use anyhow::{Context, Error};
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
        _ => match *(args.last().context("missing target")?) {
            "host" => subcommand_host(&args),
            "target" => subcommand_target(&args),
            _ => subcommand_all(&args),
        },
    }?;

    Ok(())
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
    println!("building host...");
    cmd!("cargo build").run()?;
    Ok(())
}

fn build_target() -> Result<()> {
    let _p = xshell::pushd(root_dir().join("cross"))?;
    println!("building target...");
    cmd!("cargo build --release").run()?;
    Ok(())
}

fn test_all() -> Result<()> {
    test_host()?;
    test_target()?;
    Ok(())
}

fn test_host() -> Result<()> {
    let _p = xshell::pushd(root_dir())?;
    println!("testing host...");
    cmd!("cargo test").run()?;
    Ok(())
}

fn test_target() -> Result<()> {
    let _p = xshell::pushd(root_dir().join("cross"))?;
    println!("testing target...");
    cmd!("cargo test --package self_tests").run()?;
    Ok(())
}

fn subcommand_all(args: &[&str]) -> Result<()> {
    subcommand_host(args)?;
    subcommand_target(args)?;
    Ok(())
}

fn subcommand_host(args: &[&str]) -> Result<()> {
    let _p = xshell::pushd(root_dir())?;
    cmd!("cargo {args...}").run()?;
    Ok(())
}

fn subcommand_target(args: &[&str]) -> Result<()> {
    let _p = xshell::pushd(root_dir().join("cross"))?;
    cmd!("cargo {args...}").run()?;
    Ok(())
}

fn root_dir() -> PathBuf {
    let mut xtask_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    xtask_dir.pop();
    xtask_dir
}
