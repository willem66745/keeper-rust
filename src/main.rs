
extern crate dailyschedule;
extern crate daylight;
#[macro_use] extern crate log;
extern crate log4rs;
extern crate ntpclient;
extern crate plugwise;
extern crate time;
extern crate toml;
extern crate zoneinfo;
extern crate iron;
extern crate router;
extern crate rustc_serialize;
extern crate staticfile;
extern crate mount;

mod config;
mod serial;
mod tracker;
mod ticker;
mod web;

use tracker::Tracker;
use std::env;
use std::default::Default;
use std::path::PathBuf;

const USER_CONFIG: &'static str = ".plugwise.toml";
const USER_LOGCONFIG: &'static str = ".keeper.log.toml";
const USER_WEB: &'static str = "keeper.web";

const SYSTEM_CONFIG: &'static str = "/etc/keeper/plugwise.toml";
const SYSTEM_LOGCONFIG: &'static str = "/etc/keeper/logging.toml";
const SYSTEM_WEB: &'static str = "/etc/keeper/web";

fn get_config_file(local_config: &str, system_config: &str) -> Option<PathBuf> {
    if let Some(mut homedir) = env::home_dir() {
        homedir.push(local_config);

        if homedir.exists() {
            return Some(homedir)
        }
    }

    if let Ok(mut curdir) = env::current_dir() {
        curdir.push(local_config);

        if curdir.exists() {
            return Some(curdir)
        }
    }

    let path = PathBuf::from(system_config);

    if path.exists() {
        return Some(path)
    }

    None
}

fn main() {
    let logging_config_file = get_config_file(USER_LOGCONFIG, SYSTEM_LOGCONFIG).expect("BUG: unable to find home/user directory for logging");
    let plugwise_config_file = get_config_file(USER_CONFIG, SYSTEM_CONFIG).expect("BUG: unable to find home/user directory for plugwise configuration");
    let webresources = get_config_file(USER_WEB, SYSTEM_WEB).expect("BUG: unable to find web resources");

    log4rs::init_file(logging_config_file, Default::default()).unwrap();

    let tracker = Tracker::spawn(plugwise_config_file);

    let mut web = web::Web::new();
    web.serve(tracker.get_client(), &webresources);
    tracker.teardown();
}
