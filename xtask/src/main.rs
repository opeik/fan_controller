use std::{env, path::PathBuf};
use xshell::cmd;

fn main() -> Result<(), anyhow::Error> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let args = args.iter().map(|s| &**s).collect::<Vec<_>>();

    match &args[..] {
        ["test", "all"] => test_all(),
        ["test", "host"] => test_host(),
        ["test", "target"] => test_target(),
        _ => {
            println!("USAGE cargo xtask test [all|host|target]");
            Ok(())
        }
    }
}

fn test_all() -> Result<(), anyhow::Error> {
    test_host()?;
    test_target()?;
    Ok(())
}

fn test_host() -> Result<(), anyhow::Error> {
    let _p = xshell::pushd(root_dir())?;
    cmd!("cargo test --workspace").run()?;
    Ok(())
}

fn test_target() -> Result<(), anyhow::Error> {
    let _p = xshell::pushd(root_dir().join("cross"))?;
    cmd!("cargo test --package self_tests").run()?;
    Ok(())
}

fn root_dir() -> PathBuf {
    let mut xtask_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    xtask_dir.pop();
    xtask_dir
}
