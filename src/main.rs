
extern crate dailyschedule;
extern crate daylight;
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

#[cfg(not(test))]
const CONFIG: &'static str = ".plugwise.toml";

#[cfg(not(test))]
fn main() {
    let mut configfile = env::home_dir().expect("BUG: unable to find home/user directory");
    configfile.push(CONFIG);
    let tracker = Tracker::spawn(configfile);
    tracker.join();
}
