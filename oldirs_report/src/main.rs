use clap::Parser;
use owo_colors::OwoColorize;
use std::io::BufRead;
use std::{io, sync::Arc};
use ubyte::{ByteUnit, ToByteUnit};
use users::Users;

#[derive(Parser)]
#[clap(about = "Filter, pretty-print, and summarize the output of oldirs.")]
struct Cli {
    /// username or UID to filter by
    #[clap(short, long, default_value = "")]
    user: Option<String>,
    /// minimum size to filter by
    #[clap(short, long, default_value = "0B", value_parser = parse_byte_unit)]
    size: ByteUnit,
}

fn parse_byte_unit(s: &str) -> Result<ByteUnit, String> {
    s.parse().map_err(|e: ubyte::Error| e.to_string())
}

fn parse_oldirs_line(line: String) -> anyhow::Result<(String, u32, ByteUnit)> {
    let e = || anyhow::Error::msg("malformed");
    let (s, size) = line.rsplit_once(' ').ok_or_else(e)?;
    let (path, uid) = s.rsplit_once(' ').ok_or_else(e)?;
    let bytes = size.parse().ok().ok_or_else(e)?;
    Ok((path.to_string(), uid.parse()?, bytes))
}

fn get_user(uc: &mut users::UsersCache, user: &str) -> Option<Arc<users::User>> {
    uc.get_user_by_name(user)
        .or_else(|| user.parse().ok().and_then(|uid| uc.get_user_by_uid(uid)))
}

fn main() -> anyhow::Result<()> {
    let args: Cli = Cli::parse();
    let mut total_size = ByteUnit::Byte(0);
    let mut uc = users::UsersCache::new();
    let filter_user = args.user.as_deref().and_then(|u| get_user(&mut uc, u));

    for line in io::stdin().lock().lines() {
        let (path, uid, size) = line
            .map_err(anyhow::Error::from)
            .and_then(parse_oldirs_line)?;

        // filters
        if size < args.size {
            continue;
        }
        if let Some(ref user) = filter_user {
            if user.uid() != uid {
                continue;
            }
        }

        total_size += size;

        if filter_user.is_none() {
            let colored_username = uc
                .get_user_by_uid(uid)
                .map(|u| u.name().to_string_lossy().cyan().to_string())
                .unwrap_or_else(|| uid.magenta().to_string());
            println!("{} {} {}", path, colored_username, size.yellow())
        } else {
            println!("{} {}", path, size.yellow())
        }
    }

    println!(
        "        ============================== TOTAL SIZE: {} \
                 ==============================",
        total_size.bytes().yellow()
    );
    Ok(())
}
