use std::{env, process::ExitCode};

use anyhow::Result;
use lune::Runtime;

pub(crate) mod metadata;
pub(crate) mod tracer;

use self::metadata::Metadata;

/**
    Returns whether or not the currently executing Lune binary
    is a standalone binary, and if so, the bytes of the binary.
*/
pub async fn check() -> Option<Vec<u8>> {
    let (is_standalone, patched_bin) = Metadata::check_env().await;
    if is_standalone {
        Some(patched_bin)
    } else {
        None
    }
}

/**
    Discovers, loads and executes the bytecode contained in a standalone binary.
*/
use crate::fs::ZipFileSystem;
use lune_utils::fs::FileSystem;
use std::sync::Arc;

/**
    Discovers, loads and executes the bytecode contained in a standalone binary.
*/
pub async fn run(patched_bin: impl AsRef<[u8]>) -> Result<ExitCode> {
    // The first argument is the path to the current executable
    let args = env::args().skip(1).collect::<Vec<_>>();
    let meta = Metadata::from_bytes(patched_bin).expect("must be a standalone binary");

    // Initialize filesystem from embedded ZIP data
    let zip_fs = Arc::new(ZipFileSystem::new(meta.zip_data)?);

    // Read the main entry point (init.luau)
    let main_chunk = zip_fs.read(std::path::Path::new("init.luau"))?;

    let mut rt = Runtime::new()?.with_args(args).with_fs(zip_fs)?;

    // Use a path that indicates we are at the root of the virtual filesystem
    let chunk_name = "@init.luau";

    let result = rt.run_custom(chunk_name, main_chunk).await;

    Ok(match result {
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
        Ok(values) => ExitCode::from(values.status()),
    })
}
