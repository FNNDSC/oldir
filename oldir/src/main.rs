use async_recursion::async_recursion;
use clap::Parser;
use futures::StreamExt;
use std::fs::Metadata;
use std::os::unix::prelude::MetadataExt;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio_stream::wrappers::ReadDirStream;

/// Number of concurrent recursive calls to make in each directory.
/// This value is to be passed to [StreamExt::buffered] or [StreamExt::buffer_unordered].
/// 
/// This is _not_ the same as how many "threads" to use, since each
/// recursive call to a directory will spawn another *N* green threads.
/// The growth is quadratic thus a value larger than 1 would likely
/// spiral out of control with "Too many open files" errors.
/// 
/// Besides, for several other reasons (such as organization, multi-processing)
/// it's preferable to parallelize at the top-level using GNU parallel e.g.
/// 
/// ```shell
/// find /neuro/labs/grantlab/research/ -type d -maxdepth 1 | parallel oldir
/// ```
const CONCURRENT_RECURSIVE_CALLS: usize = 1;

#[derive(Parser)]
#[clap(
    about = "Recursively find upper-most directories in which all files were not accessed in a while. All errors are reported to stderr then ignored! Symlinks are not followed."
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
    let result = oldir(args.dir, since).await.unwrap();
    for (path, metadata, size) in result {
        println!("{} {} {}", path.to_string_lossy(), metadata.uid(), size)
    }
}

async fn oldir(path: PathBuf, since: SystemTime) -> std::io::Result<Vec<(PathBuf, Metadata, u64)>> {
    let metadata = fs_err::tokio::metadata(&path).await?;
    let (ret, _) = oldir_recursive(path, metadata, since, true).await;
    Ok(ret)
}

// Super messy, deeply nested, imparative-ish code due to the limitations of Rust typing+async+recursion.
#[async_recursion]
async fn oldir_recursive(
    path: PathBuf,
    metadata: Metadata,
    since: SystemTime,
    all_older: bool
) -> (Vec<(PathBuf, Metadata, u64)>, bool) {
    if metadata.is_symlink() {
        (vec![], all_older)
    } else if metadata.is_file() {
        match metadata.accessed() {
            Ok(accessed) => {
                if accessed < since {
                    let l = metadata.len();
                    (vec![(path, metadata, l)], true)
                } else {
                    (vec![], false)
                }
            }
            Err(e) => {
                eprintln!("Error getting access time of {:?}: {:?}", path, e);
                (vec![], all_older)
            }
        }
    } else if !metadata.is_dir() {
        eprintln!(
            "Is not a file nor directory: {:?}, type: {:?}",
            path,
            metadata.file_type()
        );
        (vec![], all_older)
    } else {
        let r = tokio::fs::read_dir(&path).await.map(ReadDirStream::new);
        match r {
            Ok(rds) => {
                let (subpaths, sub_older): (Vec<Vec<(PathBuf, Metadata, u64)>>, Vec<bool>) = rds
                    .filter_map(|r| async move {
                        match r {
                            Ok(entry) => {
                                entry
                                    .metadata()
                                    .await
                                    .map_err(|e| {
                                        eprintln!(
                                            "Cannot get metadata of {:?} because {:?}",
                                            entry.path(),
                                            &e
                                        );
                                        e
                                    })
                                    .ok()
                                    .filter(|metadata| !metadata.is_symlink()) // do not follow symlinks
                                    .and_then(|metadata| Some((entry, metadata)))
                            }
                            Err(e) => {
                                // cannot print out path here, or else we'll move it
                                eprintln!("ReadDirStream produced an error: {:?}", e);
                                None
                            }
                        }
                    })
                    .map(|(entry, metadata)| oldir_recursive(entry.path(), metadata, since, all_older))
                    .buffered(CONCURRENT_RECURSIVE_CALLS)
                    .unzip()
                    .await;
                
                // let temp: Vec<Vec<&PathBuf>> = subpaths.iter().map(|v| v.iter().map(|(p, _m, _s)| p).collect()).collect();
                // dbg!(temp);
                if all_older && sub_older.into_iter().all(std::convert::identity) {
                    let total_size = subpaths.iter().flatten().map(|(_p, _m, s)| s).sum();
                    (vec![(path, metadata, total_size)], true)
                } else {
                    (subpaths.into_iter().flatten().collect(), false)
                }
            }
            Err(e) => {
                eprintln!("Unable to call read_dir on {:?} : {:?}", path, e);
                // should we panic if "Too many open files" error?
                (vec![], all_older)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{path::Path, collections::HashSet};

    use super::*;

    #[tokio::test]
    async fn test_oldir() {
        let example_dir = Path::new("example_dir");
        let five_years_ago = humantime::parse_rfc3339("2018-12-30T00:00:00Z").unwrap();
        let result = oldir(example_dir.into(), five_years_ago).await.unwrap();
        let actual: HashSet<PathBuf> = result.into_iter().map(|(p, _m, _s)| p).collect();
        let expected = ["example_dir/a/w", "example_dir/b"]
            .iter()
            .map(PathBuf::from)
            .collect();
        assert_eq!(actual, expected)
    }
}