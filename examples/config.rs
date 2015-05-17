
extern crate keeper;

use std::env;

const CONFIG: &'static str = ".plugwise.toml";

fn main() {
    let mut configfile = env::home_dir().expect("unable to find home/user directory");
    configfile.push(CONFIG);

    let result = keeper::config::Config::new(configfile).ok().expect("unable to load config");

    println!("{:?}", result);
}
