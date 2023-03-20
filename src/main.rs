use clap::{App, Arg, ArgMatches};
use env_logger::Env;
use env_logger::Target::Stdout;
use log::debug;

use id3rs::ID3Tag;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
  let args = App::new("id3-rs")
    .about("Rust based ID3 tagging")
    .arg(debug_arg())
    .arg(Arg::with_name("FILE")
      .help("File to rate")
      .required(true))
    .get_matches();

  configure_logging(&args);

  if let Some(file) = args.value_of("FILE") {
    ID3Tag::read(file)?;
  };

  Ok(())
}

pub fn debug_arg() -> Arg<'static> {
  Arg::with_name("DEBUG")
    .help("Show debug logging")
    .short('d')
    .long("debug")
}

pub fn configure_logging(args: &ArgMatches) {
  let filter = if args.is_present("DEBUG") { "debug,html5ever=info" } else { "info" };
  env_logger::Builder::from_env(Env::default().default_filter_or(filter)).target(Stdout).init();
  debug!("Debug logging");
}