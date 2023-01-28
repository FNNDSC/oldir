# oldir

Find paths containing files which have not been accessed in a long time.
Useful for doing filesystem cleanup and freeing up disk space.

## About This Document

Usage examples described below are specific to the cyber-infrastrucure of our lab, the FNNDSC.
Nonetheless, it should all make sense for _any_ UNIX filesystem.

## Background

We have a NFS share mounted on all workstations at `/neuro`,
home user directories in `/neuro/users`, and a common space
`/neuro/labs/grantlab/research`.

### Motivation

We are trying to find unused data on the FNNDSC NFS share and
move it to archival storage for the sake of freeing up space.

## How It Works

The naive approach would be to use `find` to search for files which weren't accessed anytime recently (say, in the last 2 years) by checking the file metadata/stat's `atime`.

```shell
find /neuro/labs/grantlab/research -type f -atime '+730'
```

The command above would give you an over-abundance of information – too difficult to consume.

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

If we were to run `olddir --since 5y base` we would want the output to include `base/a/w`, and `base/b`,
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

#### `oldir` Implementation Details

- Errors are printed to stderr but otherwise ignored.
- Symbolic links are not followed.
- The algorithm uses an accumulator, it cannot yield output until it has run to completion.

### `oldirs_report`

`oldirs_report` is a program which consumes the output of `oldir`, applying pretty-printing and optional filters.

#### Usage Examples

NOTE: avoid permission errors with `sudo -s` on a host with `no_root_squash` NFS privilege.

```shell
export PATH="/neuro/labs/grantlab/research/Jennings/progs/bin:$PATH"

# basic usage
oldir --since 1y /neuro/labs/grantlab/research/Ai_Others/ | oldirs_report
# filter by username and/or size
oldir --since 1y /neuro/labs/grantlab/research/Ai_Others/ | oldirs_report --user aiwern.chung --size 1GiB
```

## Data

To generate all data, I am running these commands:

```shell
export PATH="/neuro/labs/grantlab/research/Jennings/progs/bin:$PATH"

mkdir -vp data/oldir_2y/{research,users}
find /neuro/labs/grantlab/research/ -maxdepth 1 -type d | parallel --verbose 'oldir --since 2y {}/ > data/oldir_2y/research/{/}.txt 2> data/oldir_2y/research/{/}.log'
find /neuro/users/ -maxdepth 1 -type l | parallel --verbose 'oldir --since 2y {}/ > data/oldir_2y/users/{/}.txt 2> data/oldir_2y/users/{/}.log'
```

## Examples: Generate Reports

```shell
export PATH="/neuro/labs/grantlab/research/Jennings/progs/bin:$PATH"
cd /neuro/labs/grantlab/research/Jennings/for/rudolph/find_old_files/data/oldir_2y/research

# check errors
cat *.log

# report EVERYTHING!!!
cat *.txt | oldirs_report

# report your own stuff that is larger than 1GB
cat *.txt | oldirs_report --user $(whoami) --size 1GB

# get all user UIDs
cat *.txt | awk '{print $(NF-1)}' | sort | uniq

# usage by (existing) person
find /neuro/users/ -maxdepth 1 -type l \
  | parallel 's="$(cat *.txt | oldirs_report --user {/} | tail -n 1 | awk "{print \$4}")" && printf "%-30s %s\n" "{/}" "$s"' \
  > everyone.txt
cat everyone.txt

# sort the above output, starting from most usage
cat everyone.txt | ansi2txt | sort -k2 -h -r
```

## Developing

After modifying `*.rs` files, rebuild:

```shell
cargo build --release
```

For compatibility, please compile on an x86_64 Ubuntu 20.04 machine.

### Creating Examples

https://www.unixtutorial.org/how-to-update-atime-and-mtime-for-a-file-in-unix/

### Benchmarks

See also: https://github.com/jennydaman/benchmark_rust_async_recursion

```shell
hyperfine --warmup 2 \
    'find -type f -atime +730' \
    'fd --unrestricted --type f --changed-before 2y --threads 1' \
    'oldir --since 2y .'
```

##### In a 85GB HDD

```
Benchmark 1: find -type f -atime +730
  Time (mean ± σ):      1.119 s ±  0.045 s    [User: 0.368 s, System: 0.747 s]
  Range (min … max):    1.072 s …  1.209 s    10 runs
 
Benchmark 2: fd --unrestricted --type f --changed-before 2y --threads 1
  Time (mean ± σ):     814.2 ms ±  12.7 ms    [User: 261.9 ms, System: 548.8 ms]
  Range (min … max):   797.1 ms … 832.5 ms    10 runs
 
Benchmark 3: oldir --since 2y .
  Time (mean ± σ):     943.8 ms ±  42.8 ms    [User: 302.1 ms, System: 636.2 ms]
  Range (min … max):   902.7 ms … 1007.8 ms    10 runs
 
Summary
  'fd --unrestricted --type f --changed-before 2y --threads 1' ran
    1.16 ± 0.06 times faster than 'oldir --since 2y .'
    1.37 ± 0.06 times faster than 'find -type f -atime +730'
```

##### In a 485GB NFS mount

```
Benchmark 1: find -type f -atime +730
  Time (mean ± σ):      5.204 s ±  3.052 s    [User: 0.070 s, System: 0.424 s]
  Range (min … max):    2.789 s … 10.050 s    10 runs
 
  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet PC without any interferences from other programs. It might help to use the '--warmup' or '--prepare' options.
 
Benchmark 2: fd --unrestricted --type f --changed-before 2y --threads 1
  Time (mean ± σ):      5.401 s ±  1.925 s    [User: 0.142 s, System: 0.452 s]
  Range (min … max):    3.016 s …  8.266 s    10 runs
 
Benchmark 3: oldir --since 2y .
  Time (mean ± σ):      8.219 s ±  4.140 s    [User: 0.105 s, System: 0.634 s]
  Range (min … max):    3.087 s … 14.159 s    10 runs
 
Summary
  'find -type f -atime +730' ran
    1.04 ± 0.71 times faster than 'fd --unrestricted --type f --changed-before 2y --threads 1'
    1.58 ± 1.22 times faster than 'oldir --since 2y .'
```
