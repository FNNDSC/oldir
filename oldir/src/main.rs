use clap::Parser;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::SystemTime;

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

fn main() {
    let args = Cli::parse();
    let since = SystemTime::now() - **&args.since;
    let result = oldir(args.dir, since).unwrap();
    for (path, metadata, size) in result {
        println!("{} {} {}", path.to_string_lossy(), metadata.uid(), size)
    }
}

fn oldir(path: PathBuf, since: SystemTime) -> std::io::Result<Vec<(PathBuf, Metadata, u64)>> {
    let metadata = fs_err::metadata(&path)?;
    let (ret, _) = oldir_recursive(path, metadata, since, true);
    Ok(ret)
}

// Super messy, deeply nested, imperative-ish code
fn oldir_recursive(
    path: PathBuf,
    metadata: Metadata,
    since: SystemTime,
    subpath_is_older: bool,
) -> (Vec<(PathBuf, Metadata, u64)>, bool) {
    if metadata.is_symlink() {
        (vec![], subpath_is_older)
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
                (vec![], subpath_is_older)
            }
        }
    } else if !metadata.is_dir() {
        eprintln!(
            "Is not a file nor directory: {:?}, type: {:?}",
            path,
            metadata.file_type()
        );
        (vec![], subpath_is_older)
    } else {
        match fs_err::read_dir(&path) {
            Ok(rd) => {
                let (subpaths, subpaths_are_older): (Vec<Vec<(PathBuf, Metadata, u64)>>, Vec<bool>) = rd
                    .filter_map(|r| {
                        match r {
                            Ok(entry) => {
                                entry
                                    .metadata()
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
                                    .map(|metadata| (entry, metadata))
                            }
                            Err(e) => {
                                eprintln!("ReadDir of {:?} produced an error: {:?}", &path, e);
                                None
                            }
                        }
                    })
                    .map(|(entry, metadata)| {
                        oldir_recursive(entry.path(), metadata, since, subpath_is_older)
                    })
                    .unzip();

                // let temp: Vec<Vec<&PathBuf>> = subpaths.iter().map(|v| v.iter().map(|(p, _m, _s)| p).collect()).collect();
                // dbg!(temp);
                if subpath_is_older && subpaths_are_older.into_iter().all(std::convert::identity) {
                    let total_size = subpaths.iter().flatten().map(|(_p, _m, s)| s).sum();
                    (vec![(path, metadata, total_size)], true)
                } else {
                    (subpaths.into_iter().flatten().collect(), false)
                }
            }
            Err(e) => {
                eprintln!("Unable to call read_dir on {:?} : {:?}", path, e);
                // should we panic if "Too many open files" error?
                (vec![], subpath_is_older)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{collections::HashSet, path::Path};

    use super::*;

    #[test]
    fn test_oldir() {
        let example_dir = Path::new("example_dir");
        let five_years_ago = humantime::parse_rfc3339("2018-12-30T00:00:00Z").unwrap();
        let result = oldir(example_dir.into(), five_years_ago).unwrap();
        let actual: HashSet<PathBuf> = result.into_iter().map(|(p, _m, _s)| p).collect();
        let expected = ["example_dir/a/w", "example_dir/b"]
            .iter()
            .map(PathBuf::from)
            .collect();
        assert_eq!(actual, expected)
    }
}
