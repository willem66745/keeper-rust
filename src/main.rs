
extern crate dailyschedule;
extern crate daylight;
#[macro_use] extern crate log;
extern crate log4rs;
extern crate ntpclient;
extern crate plugwise;
extern crate time;
extern crate toml;
extern crate zoneinfo;

mod config;
mod serial;
mod tracker;
mod ticker;

use tracker::Tracker;
use std::env;
use std::default::Default;

const CONFIG: &'static str = ".plugwise.toml";
const LOGCONFIG: &'static str = ".keeper.log.toml";

fn main() {
    let mut logconfigfile = env::home_dir().expect("BUG: unable to find home/user directory");
    logconfigfile.push(LOGCONFIG);
    log4rs::init_file(logconfigfile, Default::default()).unwrap();
    let mut configfile = env::home_dir().expect("BUG: unable to find home/user directory");
    configfile.push(CONFIG);
    let tracker = Tracker::spawn(configfile);
    tracker.join();
}
