
extern crate keeper;
extern crate time;

use keeper::tracker::Tracker;
use keeper::ticker::Ticker;
use std::env;
use time::{at, Duration};

const CONFIG: &'static str = ".plugwise.toml";

#[derive(Copy, Clone)]
enum Keeper {
    Tick
}

fn main() {
    let mut configfile = env::home_dir().expect("BUG: unable to find home/user directory");
    configfile.push(CONFIG);
    let mut tracker = Tracker::new(configfile);
    let ticker = Ticker::spawn("nl.pool.ntp.org", Duration::seconds(10), Duration::days(1), Keeper::Tick);

    for (event, timestamp) in ticker.recv_iter() {
        match event {
            Keeper::Tick => {
                if let Some(timestamp) = timestamp {
                    println!("{}", at(timestamp).asctime());
                }
            }
        }
    }

    ticker.stop_ticker();
}
