//! Recursively find upper-most directories in which all files's atime are older than a given date.

// use async_recursion::async_recursion;
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use std::fs::Metadata;
use std::io;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio_stream::wrappers::ReadDirStream;
// use tokio_stream::StreamExt;
use async_stream::{try_stream, stream};

/// Number of parallel recursive calls to make in each directory.
/// This is _not_ the number of green threads to use, since each
/// directory will spawn another *N* green threads.
const BUFFER_UNORDERED: usize = 100;

#[tokio::main]
async fn main() {
    let since = SystemTime::now();
    let path =
        PathBuf::from("/neuro/labs/grantlab/research/Jennings/for/rudolph/find_old_files/oldirs");
    let stream = oldirs(path, since);
    pin_mut!(stream);
    while let Some(n) = stream.next().await {
        dbg!(n);
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Error on path \"{0}\": {1:?}")]
struct PathError(PathBuf, #[source] io::Error);

/// In lieu of fs-err where it can't be used.
fn fs_error_helper(p: PathBuf, e: io::Error) -> io::Error {
    io::Error::new(e.kind(), PathError(p, e))
}

fn oldirs(
    path: PathBuf,
    since: SystemTime,
) -> impl Stream<Item = Result<(PathBuf, Metadata), PathError>> {
    try_stream! {
        let metadata = fs_err::tokio::metadata(&path)
            .await
            .map_err(|e| PathError(path.clone(), e))?;

        if metadata.is_file() {
            if let Some(t) = check_single_file(path, metadata, since)? {
                yield t;
            }
            return;
        }
        if !metadata.is_dir() {
            let e = io::Error::new(io::ErrorKind::NotFound, "Not a file nor directory");
            Err(PathError(path.clone(), e))?;
        }
        let rds = tokio::fs::read_dir(&path)
            .await
            .map(ReadDirStream::new)
            .map_err(|e| PathError(path.clone(), e))?;

        let s = stream! {
            for await r in rds {
                yield match r {
                    Err(e) => {(None, Some(PathError(path.clone(), e)))},
                    Ok(entry) => {(Some(oldirs(entry.path(), since)), None)}
                }
            }
        };

        let (recursive_calls, errors): (Vec<Option<_>>, Vec<Option<PathError>>) = s.unzip().await;

        for error in errors.into_iter().filter_map(|o| o) {
            Err(error)?;
        }
        
        todo!()

    }
}


fn check_single_file(
    path: PathBuf,
    metadata: Metadata,
    since: SystemTime,
) -> Result<Option<(PathBuf, Metadata)>, PathError> {
    match metadata.accessed() {
        Ok(accessed) => {
            let o = if accessed < since {
                Some((path, metadata))
            } else {
                None
            };
            Ok(o)
        }
        Err(e) => Err(PathError(path, e)),
    }
}
