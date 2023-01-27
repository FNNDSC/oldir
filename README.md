## Motivation

We are trying to find unused data on the FNNDSC NFS share.

## How It Works

Our first approach would be to use `find` to search for files which weren't accessed anytime recently
(say, in the last 2 years) by checking the file metadata/stat's `atime`.

```shell
find /neuro/labs/grantlab/research -type f -atime '+730'
```

The command above would give you an abundance of information and it would be too difficult to consume.

### `oldir`

`oldir` is a program which recursively searches for _parent directories_ where all files have
not been accessed ever since a given _duration_.

For example, if you had the following files:

```
base/a/q  4y
base/a/w  5y
base/b/e  5y
base/b/r  5y
```

If we were to run `olddirs --since 5y base` we would want the output to include `base/a/w`, and `base/b`,
since all subpaths of those paths are 5 years or older. We would not want the output to include
`base/a` since `base/a/q` is under `base/a` but `base/a/q` is _not_ older than 5 years.
Neither would we want the output to include `base/b/e` nor `base/b/r` since we want to aggregate the
data: it's preferable to only include their parent `base/b`.

#### `oldir` Algorithm

```python
# Python-like pseudocode
def oldir(path, timestamp, subpath_is_older) -> (list[Path], bool):
    if path.is_file():
        if p.older_than(timestamp):
            return ([path], True)
        else:
            return ([], False)
    subpath_info, subpaths_are_older = unzip(oldir(subpath, timestamp, subpath_is_older) for subpath in path)
    if all(subpaths_are_older):
        # path is a dir, all immediate subpaths are either older file or
        # dir containing only older files
        return [path], True
    else:
        # path is a dir containing some things which are newer
        return flatten(subpath_info), False
```

### `dirs_report`

`dirs_report` is a program which consumes the output of `oldir`, applying pretty-printing and optional filters.

```shell
# basic usage
bin/oldir --since 1y /neuro/labs/grantlab/research/Ai_Others/ | bin/dirs_report
# filter by username and/or size
bin/oldir --since 1y /neuro/labs/grantlab/research/Ai_Others/ | bin/dirs_report --user aiwern.chung --size 1GiB
```


## Data

To generate all data, I am running these commands:

```shell
find /neuro/labs/grantlab/research/ -type d -maxdepth 1 | parallel --verbose 'bin/oldir --since 2y {} > data/oldir/research/{/}.txt 2> data/oldir/research/{/}.log'
find /neuro/users/ -type l -maxdepth 1 | parallel --verbose 'bin/oldir --since 2y {} > data/oldir/users/{/}.txt 2> data/oldir/users/{/}.log'
```

Note that `bin/oldir` needs to be run using `sudo` to avoid permission errors.

## Developing

After modifying `*.rs` files, rebuild:

```shell
cargo build --release
```
