use std::{fs, path::Path};

use chrono::{DateTime, Duration, FixedOffset, Local, TimeZone};
use dirs;
use serde::Deserialize;
use structopt::clap::AppSettings;
use structopt::StructOpt;
use toml;

use super::{enums, parsers, structs};

#[derive(Debug, StructOpt)]
#[structopt(
    about = "A simple utility for finding out what time sunrise/sunset is, and executing programs relative to these events.",
    settings = &[AppSettings::AllowLeadingHyphen]
)]
struct Cli {
    #[structopt(subcommand)]
    subcommand: Subcommand,

    #[structopt(flatten)]
    date_args: DateArgs,

    #[structopt(
        short = "l",
        long = "latitude",
        help = "Set the latitude in decimal degrees. The default is \"51.4769N\" unless overridden in ~/.config/heliocron.toml",
        requires = "longitude"
    )]
    latitude: Option<String>,

    #[structopt(
        short = "o",
        long = "longitude",
        help = "Set the longitude in decimal degrees. The default is \"0.0005W\" unless overridden in ~/.config/heliocron.toml",
        requires = "latitude"
    )]
    longitude: Option<String>,
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    Report {},

    Wait {
        #[structopt(
            help = "Choose a delay from your chosen event (see --event) in one of the following formats: {HH:MM:SS | HH:MM}. You may prepend the delay with '-' to make it negative. A negative offset will set the delay to be before the event, whilst a positive offset will set the delay to be after the event.",
            short = "o",
            long = "offset",
            default_value = "00:00:00",
            parse(from_str=parsers::parse_offset),
        )]
        offset: Duration,

        #[structopt(
            help = "Choose one of {sunrise | sunset} from which to base your delay.", 
            short = "e", 
            long = "event", 
            parse(from_str=parsers::parse_event),
            possible_values = &["sunrise", "sunset"]
        )]
        event: enums::Event,
    },
}

#[derive(Debug, StructOpt)]
struct DateArgs {
    #[structopt(short = "d", long = "date")]
    date: Option<String>,

    #[structopt(short = "f", long = "date-format", default_value = "%Y-%m-%d")]
    date_format: String,

    #[structopt(short = "t", long = "time-zone")]
    time_zone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TomlConfig {
    latitude: Option<String>,
    longitude: Option<String>,
}

impl TomlConfig {
    fn new() -> TomlConfig {
        TomlConfig {
            latitude: None,
            longitude: None,
        }
    }

    fn from_toml(config: Result<TomlConfig, toml::de::Error>) -> TomlConfig {
        match config {
            Ok(conf) => conf,
            _ => TomlConfig::new(),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub coordinates: structs::Coordinates,
    pub date: DateTime<FixedOffset>,
    pub subcommand: Option<Subcommand>,
    pub event: Option<enums::Event>,
}

impl Config {
    fn merge_toml(mut self, toml_config: TomlConfig) -> Self {
        if let (Some(latitude), Some(longitude)) = (toml_config.latitude, toml_config.longitude) {
            self.coordinates = structs::Coordinates::from_decimal_degrees(&latitude, &longitude)
        }
        self
    }

    fn merge_cli_args(mut self, cli_args: Cli) -> Self {
        // merge in location if set. Structopt requires either both or neither of lat and long to be set
        if let (Some(latitude), Some(longitude)) = (cli_args.latitude, cli_args.longitude) {
            self.coordinates = structs::Coordinates::from_decimal_degrees(&latitude, &longitude)
        }

        // set the date
        let date_args = cli_args.date_args;
        if let Some(date) = date_args.date {
            self.date = parsers::parse_date(
                Some(&date),
                &date_args.date_format,
                date_args.time_zone.as_deref(),
            );
        }

        // set the subcommand to execute
        self.subcommand = Some(cli_args.subcommand);

        self
    }
}

pub fn get_config() -> Config {
    // master function for collecting all config variables and returning a single runtime configuration

    // 0. Set up default config
    let default_config = Config {
        coordinates: structs::Coordinates::from_decimal_degrees("51.4769N", "0.0005W"),
        date: Local::today()
            .and_hms(12, 0, 0)
            .with_timezone(&FixedOffset::from_offset(Local::today().offset())),
        subcommand: None,
        event: None,
    };

    // 1. Overwrite defaults with config from ~/.config/heliocron.toml

    let path = dirs::config_dir()
        .unwrap()
        .join(Path::new("heliocron.toml"));

    let file = fs::read_to_string(path);

    let config: Config = match file {
        Ok(f) => default_config.merge_toml(TomlConfig::from_toml(toml::from_str(&f))),
        // any problems with the config file and we just continue on with the default configuration
        _ => default_config,
    };

    // 2. Add/overwrite any currently set config from CLI arguments
    let cli_args = Cli::from_args();

    let config = config.merge_cli_args(cli_args);

    config
}