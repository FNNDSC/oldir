mod prefix_buffer;

use crate::prefix_buffer::ParentPrintBuffer;
use clap::Parser;
use owo_colors::OwoColorize;
use std::io::{self, BufRead};
use ubyte::{ByteUnit, ToByteUnit};
use users::Users;

#[derive(Parser)]
#[clap(about = "Filter, pretty-print, and summarize the output of oldirs.")]
struct Cli {
    /// username or UID to filter by
    #[clap(short, long, value_parser = parse_user)]
    user: Option<users::User>,
    /// minimum size to filter by
    #[clap(short, long, default_value = "0B", value_parser = parse_byte_unit)]
    size: ByteUnit,

    /// Aggregate paths which have common parent directories
    /// if this many of them are seen in a row.
    ///
    /// Considering how `oldir` works, some of the files in
    /// a grouped parent directory are not going to be older
    /// than the `--since` argument given to `oldir`.
    /// Nonetheless, a grouped directory probably contains
    /// mostly old files which should all be reviewed.
    ///
    /// This functionality is "lossy," pass 0 to disable.
    #[clap(short, long, default_value_t = 10)]
    group: usize,
}

fn main() -> anyhow::Result<()> {
    let args: Cli = Cli::parse();
    let mut total_size = ByteUnit::Byte(0);
    let uc = users::UsersCache::new();
    let mut printer = ParentPrintBuffer::new(args.group);

    let processed_stdin = io::stdin()
        .lock()
        .lines()
        .filter_map(discard_error_from_lines)
        .filter(discard_empty_lines)
        .filter_map(parse_oldir_line_continue_on_error);
    for (path, uid, size) in processed_stdin {
        // filters
        if size < args.size {
            continue;
        }
        if let Some(ref user) = args.user {
            if user.uid() != uid {
                continue;
            }
        }

        total_size += size;

        let username = if args.user.is_none() {
            let colored_username = uc
                .get_user_by_uid(uid)
                .map(|u| u.name().to_string_lossy().cyan().to_string())
                .unwrap_or_else(|| uid.magenta().to_string());
            Some(colored_username)
        } else {
            None
        };
        printer.push(path, username, size);
    }
    printer.flush(args.user.map(|u| u.name().to_string_lossy().to_string()));

    println!(
        "        ============================== TOTAL SIZE: {} \
                 ==============================",
        total_size.bytes().yellow()
    );
    Ok(())
}

fn discard_error_from_lines(r: io::Result<String>) -> Option<String> {
    match r {
        Ok(line) => Some(line),
        Err(e) => {
            eprintln!("ERROR: {e:?}");
            None
        }
    }
}

fn discard_empty_lines(line: &String) -> bool {
    !line.trim().is_empty()
}

fn parse_oldir_line_continue_on_error(line: String) -> Option<(String, u32, ByteUnit)> {
    match parse_oldir_line(&line) {
        Ok(t) => Some(t),
        Err(e) => {
            println!("Error parsing line: {e:?}\n\t-->{line}<--");
            None
        }
    }
}

fn parse_oldir_line(line: &str) -> anyhow::Result<(String, u32, ByteUnit)> {
    let e = || anyhow::Error::msg("malformed");
    let (s, size) = line.rsplit_once(' ').ok_or_else(e)?;
    let (path, uid) = s.rsplit_once(' ').ok_or_else(e)?;
    let bytes = size.parse().ok().ok_or_else(e)?;
    Ok((path.to_string(), uid.parse()?, bytes))
}

fn parse_byte_unit(s: &str) -> Result<ByteUnit, String> {
    s.parse().map_err(|e: ubyte::Error| e.to_string())
}

fn parse_user(given_user: &str) -> Result<users::User, String> {
    if let Some(user) = users::get_user_by_name(&given_user) {
        Ok(user)
    } else {
        given_user
            .parse()
            .ok()
            .and_then(users::get_user_by_uid)
            .ok_or_else(|| format!("no such user: {}", given_user))
    }
}
