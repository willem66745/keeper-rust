
extern crate keeper;

use keeper::tracker::Tracker;
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
