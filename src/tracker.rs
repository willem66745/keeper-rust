use dailyschedule::{Handler, Schedule};
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::path;
use std::rc::Rc;
use super::config;
use super::serial;
use time::{Duration, Timespec, at_utc, at, now_utc};
use zoneinfo::ZoneInfo;

#[derive(Eq, PartialEq, Debug)]
enum Context {
    Off,
    On
}

struct Event {
    serial: serial::SerialClient,
    alias: String,
    last_on: Cell<Timespec>,
    valid_events: RefCell<BTreeSet<Timespec>>,
}

impl Event {
    fn new(alias: String, serial: serial::SerialClient) -> Event {
        Event {
            alias: alias,
            serial: serial,
            last_on: Cell::new(Timespec::new(0, 0)),
            valid_events: RefCell::new(BTreeSet::new()),
        }
    }
}

impl Handler<Context> for Event {
    /// Hint the event-handler for future events; this function will
    /// add only valid events (where on event lies before the off event) to
    /// the valid events list.
    fn hint(&self, ts: &Timespec, context: &Context) {
        match context {
            &Context::On => self.last_on.set(*ts),
            &Context::Off => {
                if *ts > self.last_on.get() {
                    let mut events = self.valid_events.borrow_mut();
                    events.insert(self.last_on.get());
                    events.insert(*ts);
                }
            }
        }
    }

    /// Perform action only when the timestamp is considered valid;
    /// Remove the current or prior timestamps from the expected timestamps.
    fn kick(&self, ts: &Timespec, context: &Context) {
        if self.valid_events.borrow().contains(ts) {
            println!("XXX: {} {:?} {}", at(*ts).asctime(), context, self.alias); // XXX
            match context {
                &Context::Off => self.serial.switch_off(&self.alias[..]),
                &Context::On => self.serial.switch_on(&self.alias[..]),
            }

            let mut events = self.valid_events.borrow_mut();

            // don't like the clone here, but keeps events mutable inside the loop
            for e in events.clone().iter().take_while(|&e| *e <= *ts) {
                events.remove(e);
            }
        }
    }
}

struct TrackerInner {
    serial: serial::SerialClient,
    config: config::Config,
    schedule: Schedule<Context, Event>,
    schedule_ref: Timespec,
    initial: bool,
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
            schedule_ref: Timespec::new(0,0),
            initial: true,
        };

        tracker.load_schedule();

        tracker
    }

    fn update_schedule(&mut self) {
        self.schedule.update_schedule(self.schedule_ref);
        self.schedule_ref = self.schedule_ref + Duration::days(1);
    }

    fn process_tick(&mut self, timestamp: Timespec) {
        if self.initial {
            self.initial = false;
            let mut tm = at_utc(timestamp);
            tm.tm_hour = 0;
            tm.tm_min = 0;
            tm.tm_sec = 0;
            tm.tm_nsec = 0;
            self.schedule_ref = tm.to_timespec();

            // fill schedule for 48 hours:
            self.update_schedule();
            self.update_schedule();
        }

        if (self.schedule_ref - timestamp) <= Duration::days(1) {
            // make sure that at least 1 day of future updates is known
            self.update_schedule();
        }

        self.schedule.kick_event(timestamp);
    }

    // XXX remove in time
    fn fast_forward(&mut self) {
        let mut now = now_utc().to_timespec();

        loop {
            match self.schedule.kick_event(now) {
                Some(next) => {
                    now = next;
                }
                None => break
            }
        }
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

    pub fn process_tick(&mut self, timestamp: Timespec) {
        self.inner.process_tick(timestamp);
    }

    // XXX remove in time
    pub fn update_schedule(&mut self) {
        self.inner.update_schedule();
    }

    // XXX remove in time
    pub fn fast_forward(&mut self) {
        self.inner.fast_forward();
    }
}

