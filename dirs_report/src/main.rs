use clap::Parser;
use dirs_info::Info;
use owo_colors::OwoColorize;
use std::io;
use std::io::BufRead;
use ubyte::{ByteUnit, ToByteUnit};

#[derive(Parser)]
struct Cli {
    /// username to filter by
    #[clap(short, long, default_value = "")]
    username: String,
    /// minimum size to filter by
    #[clap(short, long, default_value = "0B", value_parser = parse_byte_unit)]
    size: ByteUnit
}

fn parse_byte_unit(s: &str) -> Result<ByteUnit, String> {
    s.parse().map_err(|e: ubyte::Error| e.to_string())
}


fn main() {
    let args: Cli = Cli::parse();
    let mut total_size: u128 = 0;

    for line in io::stdin().lock().lines() {
        let a: anyhow::Result<Info> = line
            .map_err(anyhow::Error::from)
            .and_then(|l| serde_json::from_str(&l).map_err(anyhow::Error::from));
        match a {
            Ok(info) => {
                if !args.username.is_empty() && info.owner != args.username {
                    continue;
                }

                let size = info.size.bytes();
                if size < args.size {
                    continue;
                }

                total_size += info.size as u128;
                println!("{} {}", info.path, size.yellow())
            }
            Err(e) => eprintln!("{:?}", e),
        }
    }

    println!(
        "        ============================== TOTAL SIZE: {} \
                 ==============================",
        total_size.bytes().yellow()
    );
}
