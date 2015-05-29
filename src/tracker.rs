use dailyschedule::{Handler, Schedule};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path;
use std::rc::Rc;
use super::config;
use super::serial;
use time::{Duration, Timespec, at_utc, at};
use zoneinfo::ZoneInfo;
use ticker::Ticker;
use std::sync::mpsc::{channel, Sender};
use std::thread;

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
enum Context {
    Off,
    On
}

struct Switch {
    serial: serial::SerialClient,
    alias: String,
    last_on: Cell<Timespec>,
    state: Cell<Context>,
    valid_events: RefCell<BTreeMap<Timespec, Context>>,
    /// when "hot" perform actual relay operations
    hot: Cell<bool>,
}

impl Switch {
    fn new(alias: String, serial: serial::SerialClient) -> Switch {
        Switch {
            alias: alias,
            serial: serial,
            last_on: Cell::new(Timespec::new(0, 0)),
            state: Cell::new(Context::Off),
            valid_events: RefCell::new(BTreeMap::new()),
            hot: Cell::new(false),
        }
    }
}

impl Switch {
    fn set_switch_state(&self, state: Context) {
        self.state.set(state);
        self.dispatch_context();
    }

    fn dispatch_context(&self) {
        if self.hot.get() {
            match self.state.get() {
                Context::Off => self.serial.switch_off(&self.alias[..]),
                Context::On => self.serial.switch_on(&self.alias[..]),
            }
        }
    }

    fn make_hot(&self) {
        self.hot.set(true);
        self.dispatch_context();
    }
}

impl Handler<Context> for Switch {
    /// Hint the event-handler for future events; this function will
    /// add only valid events (where on event lies before the off event) to
    /// the valid events list.
    fn hint(&self, ts: &Timespec, context: &Context) {
        match context {
            &Context::On => self.last_on.set(*ts),
            &Context::Off => {
                if *ts > self.last_on.get() {
                    let mut events = self.valid_events.borrow_mut();
                    events.insert(self.last_on.get(), Context::On);
                    events.insert(*ts, Context::Off);
                }
            }
        }
    }

    /// Perform action only when the timestamp is considered valid;
    /// Remove the current or prior timestamps from the expected timestamps.
    fn kick(&self, ts: &Timespec, context: &Context) {
        if self.valid_events.borrow().contains_key(ts) {
            println!("XXX: {} {:?} {}", at(*ts).asctime(), context, self.alias); // XXX
            self.set_switch_state(*context);

            let mut events = self.valid_events.borrow_mut();

            // don't like the clone here, but keeps events mutable inside the loop
            for (e, _) in events.clone().iter().take_while(|&(&e, _)| e <= *ts) {
                events.remove(e);
            }
        }
    }
}

struct TrackerInner {
    serial: serial::SerialClient,
    schedule: Schedule<Context, Switch>,
    schedule_ref: Timespec,
    initial: bool,
    switches: BTreeMap<String, Rc<Switch>>,
}

impl TrackerInner {
    fn load_schedule(&mut self, config: &config::Config) {
        self.switches.clear();
        match config.device.serial_device {
            None => self.serial.connect_stub(),
            Some(ref dev) => self.serial.connect_device(&dev[..]).unwrap() // XXX
        }
        for circle in config.circles.iter() {
            self.serial.register_circle(&circle.alias, circle.mac);
            let switch = Rc::new(Switch::new(circle.alias.clone(), self.serial.clone()));
            match circle.default {
                config::CircleSetting::On => switch.set_switch_state(Context::On),
                config::CircleSetting::Off => switch.set_switch_state(Context::Off),
                config::CircleSetting::Schedule => {
                    for toggle in circle.toggles.iter() {
                        let start = toggle.start.into_dailyevent(&config.device);
                        let end = toggle.end.into_dailyevent(&config.device);

                        self.schedule.add_event(start, switch.clone(), Context::On);
                        self.schedule.add_event(end, switch.clone(), Context::Off);
                    }
                }
            }
            self.switches.insert(circle.alias.clone(), switch);
        }
    }

    fn new(config: &config::Config, zoneinfo: &ZoneInfo) -> TrackerInner {
        let schedule = Schedule::new(zoneinfo.clone());
        let serial = serial::Serial::spawn();

        // XXX: register circles / connect etc...

        let mut tracker = TrackerInner {
            schedule: schedule,
            serial: serial,
            schedule_ref: Timespec::new(0,0),
            initial: true,
            switches: BTreeMap::new(),
        };

        tracker.load_schedule(config);

        tracker
    }

    fn update_schedule(&mut self) {
        self.schedule.update_schedule(self.schedule_ref);
        self.schedule_ref = self.schedule_ref + Duration::days(1);
    }

    fn process_tick(&mut self, timestamp: Timespec) {
        if self.initial {
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

        if self.initial {
            self.initial = false;
            // configure the switch to actually set the relay (otherwise the initial kicks will
            // quickly toggle switches unintendedly
            for (_, ref mut switch) in self.switches.iter_mut() {
                switch.make_hot();
            }
        }
    }
}

#[derive(Copy, Clone)]
enum Message {
    Tick,
    Teardown,
}

pub struct Tracker {
    tx: Sender<(Message, Option<Timespec>)>,
    join: thread::JoinHandle<()>,
}

impl Tracker {
    pub fn spawn(configfile: path::PathBuf) -> Tracker {
        let zoneinfo = ZoneInfo::get_local_zoneinfo().ok().expect("BUG: not able to load local zoneinfo");
        let (tx, rx) = channel();

        let joiner = thread::spawn(move || {
            let config = config::Config::new(&configfile).unwrap(); // XXX
            let mut tracker = TrackerInner::new(&config, &zoneinfo);
            let ticker = Ticker::spawn(&config.device.ntp_server,
                                       Duration::seconds(10),
                                       Duration::days(1),
                                       Message::Tick);

            tx.send(ticker.get_sender()).ok().expect("BUG: tracker thread unable to communicate with spawner");

            for (event, timestamp) in ticker.recv_iter() {
                match event {
                    Message::Tick => {
                        if let Some(timestamp) = timestamp {
                            println!("tick: {}", at(timestamp).asctime());
                            tracker.process_tick(timestamp);
                        }
                    },
                    Message::Teardown => {
                        break;
                    },
                }
            }
            ticker.stop_ticker();
        });

        let sender = rx.recv().ok().expect("BUG: tracker thread unable to bootstrap");

        Tracker {
            tx: sender,
            join: joiner
        }
    }

    pub fn teardown(self) {
        (&self).tx.send((Message::Teardown, None)).ok().expect("BUG: not able to shutdown tracker");
        self.join();
    }

    pub fn join(self) {
        self.join.join().ok().expect("BUG: not able to join tracker");
    }
}
