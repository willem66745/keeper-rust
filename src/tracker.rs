use dailyschedule::{Handler, Schedule, DailyEvent, Context};
use std::cell::RefCell;
use std::path;
use std::rc::Rc;
use super::config;
use super::serial;
use time::Timespec;
use zoneinfo::ZoneInfo;

struct Event;

impl Handler for Event {
    fn kick(&mut self, timestamp: &Timespec, event: &DailyEvent, context: &Context) {
    }
}

struct TrackerInner {
    serial: serial::SerialClient,
    config: config::Config,
    schedule: Schedule<Event>,
}

impl TrackerInner {
    fn load_schedule(&mut self) {
        for circle in self.config.circles.iter() {
            match circle.default {
                config::CircleSetting::On => self.serial.switch_on(&circle.alias),
                config::CircleSetting::Off => self.serial.switch_off(&circle.alias),
                config::CircleSetting::Schedule => {
                    for toggle in circle.toggles.iter() {
                        let switch = Rc::new(RefCell::new(Event));
                        let start = toggle.start.into_dailyevent(&self.config.device);
                        let end = toggle.end.into_dailyevent(&self.config.device);

                        self.schedule.add_event(start, switch.clone(), Context(0));
                        self.schedule.add_event(end, switch.clone(), Context(0));
                    }
                }
            }
        }
    }

    fn new(configfile: &path::PathBuf, zoneinfo: &ZoneInfo) -> TrackerInner {
        let config = config::Config::new(configfile).unwrap(); // XXX
        let schedule = Schedule::<Event>::new(zoneinfo.clone());
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

        //tracker.reschedule();

        tracker
    }

//    pub fn update_config(&mut self, config: config::Config) {
//        self.config = config;
//        self.schedule = Schedule::<'a, _>::new(self.zoneinfo.clone());
//        self.reschedule();
//    }

//    fn reschedule(&self) {
//    }
}

