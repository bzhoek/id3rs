use clap::{Arg, ArgAction, ArgMatches, Command};
use env_logger::Env;
use env_logger::Target::Stdout;
use id3rs::{ID3rs, ID3HEADER_SIZE};
use log::{debug, warn};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
  let args = Command::new("id3-rs")
    .about("Rust based ID3 tagging")
    .subcommand_required(true)
    .arg(debug_arg())
    .subcommand(Command::new("check")
      .about("Check MP3 frame right after header")
      .arg(Arg::new("FILE")
        .required(true)))
    .subcommand(Command::new("info")
      .about("Display ID3 information")
      .arg(Arg::new("FILE")
        .required(true)))
    .get_matches();

  configure_logging(&args);

  match args.subcommand() {
    Some(("check", sub)) => {
      let filepath = sub.get_one::<String>("FILE").unwrap();
      let id3 = ID3rs::read(filepath)?;
      let word = first_frame(id3)?;
      if word != 0xFFFB {
        warn!("{} does not start with MP3 frame", filepath);
        std::process::exit(1);
      }
    }
    Some(("info", sub)) => {
      let filepath = sub.get_one::<String>("FILE").unwrap();
      let id3 = ID3rs::read(filepath)?;
      println!("  File: {:?}", filepath);
      if let Some(title) = id3.title() {
        println!(" Title: {}", title);
      }
      if let Some(artist) = id3.artist() {
        println!("Artist: {}", artist);
      }
      if let Some(version) = id3.subtitle() {
        if !version.is_empty() {
          println!("Version: {}", version);
        }
      }
      println!("Offset: {:#06X}", id3.header_size + ID3HEADER_SIZE);
    }
    _ => unreachable!(),
  }

  Ok(())
}

fn first_frame(tag: ID3rs) -> Result<u16> {
  let mut file = File::open(tag.path)?;
  if tag.header_size > 0 {
    file.seek(SeekFrom::Start(ID3HEADER_SIZE + tag.header_size))?;
  }
  let mut buffer = [0; 2usize];
  file.read_exact(&mut buffer).unwrap();
  let word: u16 = ((buffer[0] as u16) << 8) + buffer[1] as u16;
  Ok(word)
}

fn debug_arg() -> Arg {
  Arg::new("DEBUG")
    .help("Show debug logging")
    .short('d')
    .long("debug")
    .action(ArgAction::SetTrue)
}

fn configure_logging(args: &ArgMatches) {
  let filter = if args.get_flag("DEBUG") { "debug" } else { "info" };
  env_logger::Builder::from_env(Env::default().default_filter_or(filter)).target(Stdout).init();
  debug!("Debug logging");
}