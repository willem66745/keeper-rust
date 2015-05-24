extern crate keeper;

use keeper::tracker::Tracker;
use std::env;

const CONFIG: &'static str = ".plugwise.toml";

fn main() {
    let mut configfile = env::home_dir().expect("BUG: unable to find home/user directory");
    configfile.push(CONFIG);

    let mut tracker = Tracker::new(configfile);
    for _ in 0..365 {
        tracker.update_schedule();
    }
    tracker.fast_forward();
}
