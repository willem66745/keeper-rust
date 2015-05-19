// This module loads the configuration file

use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path;
use std::result;
use toml;

const CONFIG_HEAD: &'static str = "config";
const CONFIG_DEVICE: &'static str = "device";
const CIRCLE_MAC: &'static str = "mac";
const CIRCLE_DEFAULT: &'static str = "default";

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    MissingEventSpecifier(String),
    WrongEventSpecifier(String),
    MissingStartEvent(String),
    MissingEndEvent(String),
    ScheduleExpected(String),
    InvalidMac(String),
    InvalidDefault(String),
    InvalidToml,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => fmt::Display::fmt(err, f),
            _ => write!(f, "Keeper configuraton error"),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref err) => error::Error::description(err),
            _ => "Keeper configuration error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref err) => err.cause(),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Fixed(u8, u8),
    Fuzzy((u8, u8), (u8, u8)),
    Sunrise(u16),
    Sunset(u16)
}

impl Event {
    fn time_in_a_day(value: &toml::Value) -> Option<(u8, u8)> {
        let mapped: Option<Vec<i64>> = value.as_slice()
                                            .map(|arr| arr.iter()
                                                          .filter_map(|x|x.as_integer())
                                                          .collect());
        mapped.map_or(None, |v| {
            match v.len() {
                2 => {
                    let (h,m) = (v[0], v[1]);
                    if h < 0 || h >= 24 {
                        None
                    } else if m < 0 || m >= 60 {
                        None
                    } else {
                        Some((h as u8, m as u8))
                    }
                },
                _ => None
            }
        })
    }

    fn new(key: &str, value: &toml::Value) -> Result<Event> {
        let specifier = try!(key.split("_").last().ok_or(
                Error::MissingEventSpecifier(key.into())));

        match specifier {
            "fixed" => Event::time_in_a_day(value).map(|(f,s)| Event::Fixed(f,s)).ok_or(
                Error::WrongEventSpecifier("fixed must hold a array of two integers".into())),
            "fuzzy" => value.as_slice().map_or(None, |s| {
                    match s.len() {
                        2 => {
                            let first = Event::time_in_a_day(&s[0]);
                            let second = Event::time_in_a_day(&s[1]);
                            first.map_or(None, |f| second.map(|s| Event::Fuzzy(f,s)))
                        },
                        _ => None
                    }
                }).ok_or(Error::WrongEventSpecifier("fuzzy must hold a array of two arrays of two integers".into())),
            "sunrise" => value.as_integer().map(|i| Event::Sunrise(i as u16)).ok_or(
                Error::WrongEventSpecifier("sunrise must only hold one integer (variance in minutes)".into())),
            "sunset" => value.as_integer().map(|i| Event::Sunset(i as u16)).ok_or(
                Error::WrongEventSpecifier("sunset must only hold one integer (variance in minutes)".into())),
            _ => Err(Error::WrongEventSpecifier("unsupported specifier".into()))
        }
    }
}

#[derive(Debug)]
pub struct Toggle {
    pub alias: String,
    pub start: Event,
    pub end: Event
}

impl Toggle {
    fn new(alias: &str, table: &toml::Table) -> Result<Toggle> {
        let start = try!(table.iter().find(|&(k,_)| k.starts_with("start_")).map_or(
                Err(Error::MissingStartEvent(alias.into())),
                |(k,v)| Event::new(&k[..], v)));
        let end = try!(table.iter().find(|&(k,_)| k.starts_with("end_")).map_or(
                Err(Error::MissingEndEvent(alias.into())),
                |(k,v)| Event::new(&k[..], v)));
        Ok(Toggle {
            alias: alias.into(),
            start: start,
            end: end
        })
    }
}

#[derive(Debug)]
pub enum CircleSetting {
    Off,
    On,
    Schedule
}

impl CircleSetting {
    fn new(setting_as_str: &str) -> Option<CircleSetting> {
        match setting_as_str {
            "off" => Some(CircleSetting::Off),
            "on" => Some(CircleSetting::On),
            "schedule" => Some(CircleSetting::Schedule),
            _ => None
        }
    }
}

#[derive(Debug)]
pub struct Circle {
    pub alias: String,
    pub mac: u64,
    pub default: CircleSetting,
    pub toggles: Vec<Toggle>
}

impl Circle {
    fn new(alias: &str, table: &toml::Table) -> Result<Circle> {
        let mut mac = None;
        let mut default = None;
        let mut toggles = vec![];

        for (k, v) in table {
            match &k[..] {
                CIRCLE_MAC => {
                    mac = v.as_str().map_or(None, |s| u64::from_str_radix(s, 16).ok());
                },
                CIRCLE_DEFAULT => {
                    default = v.as_str().map_or(None, |s| CircleSetting::new(s));
                },
                _ => {
                    let toggle = try!(v.as_table().map_or(
                            Err(Error::ScheduleExpected(alias.into())),
                            |t| Toggle::new(&k[..], t)));
                    toggles.push(toggle);
                }
            }
        }

        Ok(Circle {
            alias: alias.into(),
            mac: try!(mac.ok_or(Error::InvalidMac(alias.into()))),
            default: try!(default.ok_or(Error::InvalidDefault(alias.into()))),
            toggles: toggles
        })
    }
}

#[derive(Debug)]
pub struct Device {
    pub serial_device: String
}

impl Device {
    fn new(table: &toml::Table) -> Option<Device> {
        let mut result = None;

        for (k, v) in table {
            match &k[..] {
                CONFIG_DEVICE => {
                    if let Some(string) = v.as_str() {
                        result = Some(Device{
                            serial_device: string.into()
                        });
                    }
                },
                _ => {}
            }
        }

        result
    }
}

#[derive(Debug)]
pub struct Config {
    pub device: Option<Device>,
    pub circles: Vec<Circle>
}

impl Config {
    pub fn new(configfile: path::PathBuf) -> Result<Config> {
        let mut result = Config {
            device: None,
            circles: vec![]
        };

        let mut config = String::new();
        let mut file = try!(fs::File::open(configfile));
        try!(file.read_to_string(&mut config));
        let config = try!(toml::Parser::new(&config).parse().ok_or(Error::InvalidToml));

        for (k,v) in config {
            if let Some(table) = v.as_table() {
                match &k[..] {
                    CONFIG_HEAD => {
                        result.device = Device::new(table);
                    },
                    _ => {
                        result.circles.push(try!(Circle::new(&k[..], table)));
                    }
                }
            }
        }

        Ok(result)
    }
}