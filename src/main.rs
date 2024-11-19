use clap::{Arg, Command};
use id3rs::Result;
use id3rs::{ID3rs, ID3HEADER_SIZE};
use log::info;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use ursual::{configure_logging, debug_arg, verbose_arg};

fn main() -> Result<()> {
  let args = Command::new("id3-rs")
    .about("Rust based ID3 tagging")
    .subcommand_required(true)
    .arg(debug_arg())
    .arg(verbose_arg())
    .subcommand(Command::new("check").about("Check MP3 frame right after header").arg(Arg::new("FILE").required(true)))
    .subcommand(Command::new("info").about("Display ID3 information").arg(Arg::new("FILE").required(true)))
    .get_matches();

  configure_logging(&args);
  let verbose = args.get_flag("VERBOSE");

  match args.subcommand() {
    Some(("check", sub)) => {
      let filepath = sub.get_one::<String>("FILE").unwrap();
      let id3 = ID3rs::read(filepath)?;
      check_first_frame(&id3)?;
      if verbose {
        info!("{} starts with MP3 frame", filepath);
      }
    }
    Some(("info", sub)) => {
      let filepath = sub.get_one::<String>("FILE").unwrap();
      let id3 = ID3rs::read(filepath)?;
      println!("  File: {:?}", filepath);
      print("  Title", id3.title());
      print(" Artist", id3.artist());
      print("Version", id3.subtitle());
      let size = id3.header_size + ID3HEADER_SIZE;
      println!(" Offset: {:#06X} {}", size, size);
      check_first_frame(&id3)?;
    }
    _ => unreachable!(),
  }

  Ok(())
}

fn check_first_frame(id3: &ID3rs) -> Result<()> {
  let word = first_frame(id3)?;
  if word != 0xFFFB {
    return Err(format!("{:?} does not start with MP3 frame", &id3.path).into());
  }
  Ok(())
}

fn print(header: &str, option: Option<&str>) {
  if let Some(value) = option {
    if !value.is_empty() {
      println!("{}: {}", header, value);
    }
  }
}

fn first_frame(tag: &ID3rs) -> Result<u16> {
  let mut file = File::open(tag.path.clone())?;
  if tag.header_size > 0 {
    file.seek(SeekFrom::Start(ID3HEADER_SIZE + tag.header_size))?;
  }
  let mut buffer = [0; 2usize];
  file.read_exact(&mut buffer).unwrap();
  let word: u16 = ((buffer[0] as u16) << 8) + buffer[1] as u16;
  Ok(word)
}
