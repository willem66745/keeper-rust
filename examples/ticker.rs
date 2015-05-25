
extern crate ntpclient;
extern crate time;

use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use time::{Timespec, at, Duration, precise_time_ns};

const BILLION: u64 = 1_000_000_000;

// NOTE: obsolete when Rust starts to support UDP receive timeout
struct NtpFetcher {
    server: String,
    last_sync: Arc<Mutex<(Timespec, u64)>>,
    poll: Duration,
    last_poll: u64,
}

impl NtpFetcher {
    fn new(server: String, ntp_poll: Duration) -> NtpFetcher {
        let mut ntp = NtpFetcher {
            server: server,
            last_sync: Arc::new(Mutex::new((Timespec::new(0,0), 0))),
            poll: ntp_poll,
            last_poll: 0,
        };

        ntp.consider_poll_ntp();
        ntp
    }

    fn consider_poll_ntp(&mut self) {
        if let Ok(ref lock) = self.last_sync.lock() {
            let (_, ref_time) = **lock;
            let poll_interval = if ref_time == 0 {
                // when never NTP timestamp was received, try again after 10 minutes
                Duration::minutes(10)
            } else {
                self.poll
            };

            let curr = precise_time_ns();
            let must_poll = (self.last_poll == 0) ||
                (((curr - self.last_poll) / BILLION) >= poll_interval.num_seconds() as u64);

            if must_poll {
                self.last_poll = curr;
                let sync = self.last_sync.clone();
                let server = self.server.clone();

                thread::spawn(move || {
                    println!("receiving timestamp");
                    if let Ok(ts) = ntpclient::retrieve_ntp_timestamp(&server[..]) {
                        if let Ok(ref mut lock) = sync.lock() {
                            **lock = (ts, precise_time_ns());
                        }
                    }
                });
            }
        }
    }

    fn get_timespec(&mut self) -> Option<Timespec> {
        self.consider_poll_ntp();
        match self.last_sync.lock() {
            Ok(ref lock) => {
                let (sync_ts, ref_time) = **lock;

                match ref_time {
                    0 => None,
                    _ => {
                        let ns = precise_time_ns() - ref_time;
                        Some(sync_ts + Duration::nanoseconds(ns as i64))
                    }
                }
            },
            Err(_) => None,
        }
    }
}

struct Ticker<C> {
    rx: Receiver<(C, Option<Timespec>)>,
    tx: Sender<(C, Option<Timespec>)>,
    joiner: thread::JoinHandle<()>,
    leave_guard: Arc<(Mutex<bool>, Condvar)>,
}

impl<C> Ticker<C> where C: Send + Copy + 'static {
    fn spawn(server: &str, tick_interval: Duration, ntp_poll: Duration, event: C) -> Ticker<C> {
        let (tx, rx) = channel();
        let leave_guard = Arc::new((Mutex::new(true), Condvar::new()));
        let waiter = leave_guard.clone();
        let cloned_tx = tx.clone();
        let server = server.into();

        let joiner = thread::spawn(move || {
            let mut ntp = NtpFetcher::new(server, ntp_poll);
            let &(ref lock, ref cvar) = &*waiter;
            let mut leaver = lock.lock().ok().expect("BUG: mutex cannot claimed inside thread");

            // this loop sends the NTP synchronized timestamp to receiving end of the channel
            while *leaver {
                let (new_leaver, _) =
                    cvar.wait_timeout_ms(leaver,
                                         tick_interval.num_milliseconds() as u32).ok().expect(
                                             "BUG: unexpected error during wait");

                leaver = new_leaver;

                if *leaver {
                    if let Some(ts) = ntp.get_timespec() {
                        tx.send((event, Some(ts))).ok().expect("BUG: cannot send timestamp");
                    }
                }
            }
        });

        Ticker {
            rx: rx,
            tx: cloned_tx,
            joiner: joiner,
            leave_guard: leave_guard,
        }
    }

    fn stop_ticker(self) {
        let &(ref lock, ref cvar) = &*(self.leave_guard);
        let mut leaver = lock.lock().ok().expect("BUG: cannot claim mutex during stop_ticker");
        *leaver = false;
        drop(leaver);
        cvar.notify_all();
        let _ = self.joiner.join();
    }

    fn get_transmitter(&self) -> Sender<(C, Option<Timespec>)> {
        self.tx.clone()
    }
}

#[derive(Copy, Clone)]
enum Dummy {
    Dummy
}

fn main() {
    let ticker = Ticker::spawn("nl.pool.ntp.org", Duration::seconds(1), Duration::days(1), Dummy::Dummy);
    let _ = ticker.get_transmitter(); // suppress warning

    let mut teller = 0; // XXX

    loop {
        let (_, timestamp) = ticker.rx.recv().unwrap();

        if let Some(timestamp) = timestamp {
            println!("{}", at(timestamp).asctime());
        }

        teller = teller + 1;
        if teller >= 5 {
            ticker.stop_ticker();
            break;
        }
    }
}
