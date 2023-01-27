use async_recursion::async_recursion;
use clap::Parser;
use futures::StreamExt;
use std::fs::Metadata;
use std::os::unix::prelude::MetadataExt;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio_stream::wrappers::ReadDirStream;

/// Number of parallel recursive calls to make in each directory.
/// This is _not_ the number of green threads to use, since each
/// directory will spawn another *N* green threads.
const BUFFER_UNORDERED: usize = 100;

#[derive(Parser)]
#[clap(
    about = "Recursively find upper-most directories in which all files were not accessed in a while. All errors are reported to stderr then ignored!"
)]
struct Cli {
    /// duration: files not accessed since this long ago are reported
    #[clap(short, long)]
    since: humantime::Duration,
    /// Directory to search
    dir: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let since = SystemTime::now() - **&args.since;
    let result = oldirs(args.dir, since).await;
    for (path, metadata, size) in result {
        println!("{} {} {}", path.to_string_lossy(), metadata.uid(), size)
    }
}

// Super messy, deeply nested, imparative-ish code due to the limitations of Rust typing+async+recursion.
#[async_recursion]
async fn oldirs(path: PathBuf, since: SystemTime) -> Vec<(PathBuf, Metadata, u64)> {
    match fs_err::tokio::metadata(&path).await {
        Ok(metadata) => {
            if metadata.is_file() {
                match metadata.accessed() {
                    Ok(accessed) => {
                        if accessed < since {
                            let l = metadata.len();
                            vec![(path, metadata, l)]
                        } else {
                            vec![]
                        }
                    }
                    Err(e) => {
                        eprintln!("Error getting access time of {:?}: {:?}", path, e);
                        vec![]
                    }
                }
            } else if !metadata.is_dir() {
                eprintln!(
                    "Is not a file nor directory: {:?}, type: {:?}",
                    path,
                    metadata.file_type()
                );
                vec![]
            } else {
                let r = tokio::fs::read_dir(&path).await.map(ReadDirStream::new);
                match r {
                    Ok(rds) => {
                        let subpaths: Vec<Vec<(PathBuf, Metadata, u64)>> = rds
                            .filter_map(|r| async move {
                                match r {
                                    Ok(entry) => Some(entry),
                                    Err(e) => {
                                        // cannot print out path here, or else we'll move it
                                        eprintln!("ReadDirStream produced an error: {:?}", e);
                                        None
                                    }
                                }
                            })
                            .map(|entry| oldirs(entry.path(), since))
                            .buffer_unordered(BUFFER_UNORDERED)
                            .collect()
                            .await;
                        if subpaths.iter().all(|v| v.len() <= 1) {
                            let total_size = subpaths.iter().flatten().map(|(_p, _m, s)| s).sum();
                            vec![(path, metadata, total_size)]
                        } else {
                            subpaths.into_iter().flatten().collect()
                        }
                    }
                    Err(e) => {
                        eprintln!("Unable to call read_dir on {:?} : {:?}", path, e);
                        vec![]
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{:?}", e);
            vec![]
        }
    }
}
