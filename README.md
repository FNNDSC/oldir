## Motivation

We are trying to find unused data on the FNNDSC NFS share.

## The Data

Basic `find` looks like:

```shell
sudo find -L /neuro/labs/grantlab/research -type f -atime '+730'
```

This `find` command finds files which have not been accessed in the past 2 years.
`sudo` is needed to each all files (since some you might not have read access to).

These `find` commands were ran in `tmux`.

### Parallelizing `find`

Run parallel subprocesses of `find` for each immediate subdirectory of `/neuro/labs/grantlab/research`

```shell
mkdir -vp data/research
find /neuro/labs/grantlab/research -maxdepth 1 -type d | parallel --verbose 'find {} -type f -atime "+730" > data/research/{/}.txt'
```

For users:

```shell
mkdir -vp data/users
find /neuro/users/ -maxdepth 1 -type l | parallel --dry-run 'find -L {}/ -type f -atime "+730" > data/users/{/}.txt'
```

Join all the results together:

```shell
cat data/research/*.txt > data/everything_research.txt
cat 
```

## How It Works

Next, we process the "raw" data using the program `dirs_info`. It identifies parent directories
which contain files belonging to the same directory. The parent directory's path, owner, and size
are spat out as JSON.

The JSON output of `dirs_info` can be parsed by `dirs_report` which pretty-prints the size,
aggregates the sum of sizes, and optionally filters based on username.

## Usage Examples

These Rust programs were compiled on `centurion`, which runs x86_64 Ubuntu 22.04.1 LTS.
`dirs_info` and `dirs_report` are only going to work on similar machines.

```shell
# REPORT EVERYTHING!!!
cat older_than_5y_in_research.txt | bin/dirs_info /neuro/labs/grantlab/research | bin/dirs_report

# ok, that was a bad idea... From now on, let's just look at part of the data first.

# get a list of usernames:
head -n 1000 older_than_5y_in_research.txt | bin/dirs_info /neuro/labs/grantlab/research | jq -r '.owner' | sort | uniq

# report folders by daniel.haehn:
head -n 1000 older_than_5y_in_research.txt | sort | bin/dirs_info /neuro/labs/grantlab/research 2> /dev/null | bin/dirs_report --username daniel.haehn

# analyze ayse.tanritanir's home folder:
head -n 1000 older_than_5y_in_users.txt | grep '^/neuro/users/ayse.tanritanir' | sudo bin/dirs_info /neuro/users/ayse.tanritanir | bin/dirs_report --username ayse.tanritanir
```


## Developing

After modifying `*.rs` files, rebuild:

```shell
cargo build --release
```
