//! Finds parent folders from a sorted list of files passed via stdin.
use dirs_info::Info;
use file_owner::PathExt;
use std::io::BufRead;
use std::path::Path;
use std::{env, fs, io};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let base_path = args
        .get(1)
        .map(|s| {
            if s.ends_with('/') {
                s.to_string()
            } else {
                format!("{}/", s)
            }
        })
        .ok_or_else(|| anyhow::Error::msg(format!("usage: {} /SEARCHED/BASE/PATH", args[0])))?;

    let mut common_prefix = "".to_string();

    for line in io::stdin().lock().lines() {
        let line = line?;
        let relative_path = &line[base_path.len()..];

        if common_prefix.is_empty() {
            common_prefix = dirname(relative_path);
        } else {
            let next_prefix = common_prefix_between(&common_prefix, relative_path);
            if next_prefix.is_empty() {
                print_dir_details(&base_path, &common_prefix);
            }
            common_prefix = next_prefix
        }
    }

    if !common_prefix.is_empty() {
        print_dir_details(&base_path, &common_prefix);
    }

    Ok(())
}

fn dirname(path: &str) -> String {
    if let Some((dir, _file)) = path.rsplit_once('/') {
        dir
    } else {
        path
    }
    .to_string()
}

fn common_prefix_between<'a>(x: &'a str, y: &'a str) -> String {
    if x.is_empty() {
        return x.to_string();
    }
    if x.len() > y.len() {
        return common_prefix_between(y, x);
    }

    let s: Vec<&str> = x.split('/').collect();
    for (i, (l, r)) in s.iter().zip(y.split('/')).enumerate() {
        if *l != r {
            return s[0..i].join("/");
        }
    }
    x.to_string()
}

/// Prints a directory's path, owner, and size.
fn print_dir_details(base_path: &str, rel_folder: &str) {
    let path = format!("{}{}", base_path, rel_folder);
    let owner = path
        .owner()
        .and_then(|o| o.name().map(|name| name.unwrap_or(o.id().to_string())))
        .unwrap_or_else(|e| {
            eprintln!("ERROE: cannot get owner of {} because {:?}", path, e);
            "ERROR_OWNER".to_string()
        });
    let size = get_size(&path).unwrap_or_else(|e| {
        eprintln!("ERROR: cannot size of {} because {:?}", path, e);
        0
    });
    let info = Info { path, size, owner };
    println!("{}", serde_json::to_string(&info).unwrap())
}

/// Gets the size of a path, equivalent to `du --bytes -s <path>`.
///
/// WARNING: any errors which happen during recusion will cause the entire thing to fail!
///
/// Copied from
/// https://github.com/webdesus/fs_extra/blob/bb12c0d7cc286614fad8b59b8d9961fac8b7bde2/src/dir.rs#L786-L816
///
/// An important fix was made recently in
/// https://github.com/webdesus/fs_extra/commit/1bebf1ebe44732f2e04a1176047dd323bb819c7a
/// however it has not been released yet.
pub fn get_size<P>(path: P) -> io::Result<u64>
where
    P: AsRef<Path>,
{
    // Using `fs::symlink_metadata` since we don't want to follow symlinks,
    // as we're calculating the exact size of the requested path itself.
    let path_metadata = path.as_ref().symlink_metadata()?;

    let mut size_in_bytes = 0;

    if path_metadata.is_dir() {
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            // `DirEntry::metadata` does not follow symlinks (unlike `fs::metadata`), so in the
            // case of symlinks, this is the size of the symlink itself, not its target.
            let entry_metadata = entry.metadata()?;

            if entry_metadata.is_dir() {
                // The size of the directory entry itself will be counted inside the `get_size()` call,
                // so we intentionally don't also add `entry_metadata.len()` to the total here.
                size_in_bytes += get_size(entry.path())?;
            } else {
                size_in_bytes += entry_metadata.len();
            }
        }
    } else {
        size_in_bytes = path_metadata.len();
    }

    Ok(size_in_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_prefix_between() {
        assert_eq!(&common_prefix_between("", "bubba"), "");
        assert_eq!(&common_prefix_between("bubba", "bubba"), "bubba");
        assert_eq!(&common_prefix_between("bub/bles", "bub"), "bub");
        assert_eq!(&common_prefix_between("bub", "bub/bles"), "bub");
        assert_eq!(&common_prefix_between("hello", "bubbles"), "");
        assert_eq!(&common_prefix_between("bub/bles", "bub/blez"), "bub");
        assert_eq!(
            &common_prefix_between("bub/bles/sushi", "bub/bles/sashimi"),
            "bub/bles"
        );
    }
}
