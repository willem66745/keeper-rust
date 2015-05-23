use dailyschedule::{Handler, Schedule, DailyEvent};
use std::path;
use std::rc::Rc;
use super::config;
use super::serial;
use time::Timespec;
use zoneinfo::ZoneInfo;

#[derive(Eq, PartialEq)]
enum Context {
    Off,
    On
}

struct Event {
    serial: serial::SerialClient,
    alias: String
}

impl Event {
    fn new(alias: String, serial: serial::SerialClient) -> Event {
        Event {
            alias: alias,
            serial: serial
        }
    }
}

impl Handler<Context> for Event {
    fn kick(&self, _: &Timespec, _: &DailyEvent, context: &Context) {
        match context {
            &Context::Off => self.serial.switch_off(&self.alias[..]),
            &Context::On => self.serial.switch_on(&self.alias[..]),
        }
    }
}

struct TrackerInner {
    serial: serial::SerialClient,
    config: config::Config,
    schedule: Schedule<Context, Event>,
}

impl TrackerInner {
    fn load_schedule(&mut self) {
        for circle in self.config.circles.iter() {
            match circle.default {
                config::CircleSetting::On => self.serial.switch_on(&circle.alias),
                config::CircleSetting::Off => self.serial.switch_off(&circle.alias),
                config::CircleSetting::Schedule => {
                    for toggle in circle.toggles.iter() {
                        let switch = Rc::new(Event::new(circle.alias.clone(), self.serial.clone()));
                        let start = toggle.start.into_dailyevent(&self.config.device);
                        let end = toggle.end.into_dailyevent(&self.config.device);

                        self.schedule.add_event(start, switch.clone(), Context::On);
                        self.schedule.add_event(end, switch.clone(), Context::Off);
                    }
                }
            }
        }
    }

    fn new(configfile: &path::PathBuf, zoneinfo: &ZoneInfo) -> TrackerInner {
        let config = config::Config::new(configfile).unwrap(); // XXX
        let schedule = Schedule::new(zoneinfo.clone());
        let serial = serial::Serial::spawn();

        let mut tracker = TrackerInner {
            config: config,
            schedule: schedule,
            serial: serial,
        };

        tracker.load_schedule();

        tracker
    }
}

pub struct Tracker {
    zoneinfo: ZoneInfo,
    path: path::PathBuf,
    inner: TrackerInner,
}

impl Tracker {
    pub fn new(configfile: path::PathBuf) -> Tracker {
        let zoneinfo = ZoneInfo::get_local_zoneinfo().ok().expect("BUG: not able to load local zoneinfo");
            
        let tracker = Tracker {
            inner: TrackerInner::new(&configfile, &zoneinfo),
            path: configfile,
            zoneinfo: zoneinfo,
        };

        tracker
    }
}

