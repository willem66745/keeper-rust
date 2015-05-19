
extern crate keeper;

use std::env;

const CONFIG: &'static str = ".plugwise.toml";

fn main() {
    let mut configfile = env::home_dir().expect("BUG: unable to find home/user directory");
    configfile.push(CONFIG);

    let config = keeper::config::Config::new(configfile).ok()
                                                        .expect("BUG: unable to load config");

    let serial = keeper::serial::Serial::spawn();

    let _ = serial.connect_device("wahwahwah");
    serial.connect_stub();

    for circle in config.circles {
        serial.register_circle(&circle.alias, circle.mac);
    }

    serial.hangup();
}
