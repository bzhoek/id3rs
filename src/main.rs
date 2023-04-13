use clap::{Arg, ArgAction, ArgMatches, Command};
use env_logger::Env;
use env_logger::Target::Stdout;
use log::debug;

use id3rs::ID3rs;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
  let args = Command::new("id3-rs")
    .about("Rust based ID3 tagging")
    .arg(debug_arg())
    .arg(Arg::new("FILE")
      .help("File to rate")
      .required(true))
    .get_matches();

  configure_logging(&args);

  if let Some(file) = args.get_one::<String>("FILE") {
    let tag = ID3rs::read(file)?;
    println!("Title: {:?}", tag.title());
    println!("Version: {:?}", tag.subtitle());
    println!("Artist: {:?}", tag.artist());
  };

  Ok(())
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