
use ntpclient::retrieve_ntp_timestamp;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::{channel, Sender, Receiver, Iter};
use std::thread;
use time::{Timespec, Duration, precise_time_ns};

const BILLION: u64 = 1_000_000_000;

// NOTE: obsolete when Rust starts to support UDP receive timeout
struct NtpFetcher {
    server: String,
    last_sync: Arc<Mutex<(Timespec, u64)>>,
    poll: Duration,
    last_poll: u64,
    ntp_update: Arc<(Mutex<bool>, Condvar)>,
}

impl NtpFetcher {
    fn new(server: String,
           ntp_poll: Duration,
           ntp_update: Arc<(Mutex<bool>, Condvar)>) -> NtpFetcher {
        let mut ntp = NtpFetcher {
            server: server,
            last_sync: Arc::new(Mutex::new((Timespec::new(0,0), 0))),
            poll: ntp_poll,
            last_poll: 0,
            ntp_update: ntp_update,
        };

        ntp.consider_poll_ntp();
        ntp
    }

    fn consider_poll_ntp(&mut self) {
        if let Ok(ref lock) = self.last_sync.lock() {
            let (_, ref_time) = **lock;
            let poll_interval = if ref_time == 0 {
                // when never NTP timestamp was received, try again after 1 minute
                Duration::minutes(1)
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
                let update = self.ntp_update.clone();

                let join = thread::spawn(move || {
                    if let Ok(ts) = retrieve_ntp_timestamp(&server[..]) {
                        if let Ok(ref mut lock) = sync.lock() {
                            let (_, ref_time) = **lock;
                            **lock = (ts, precise_time_ns());
                            if ref_time == 0 {
                                // notify ntp listener that a initial ntp result is known
                                let &(_, ref cvar) = &*update;
                                cvar.notify_all();
                            }
                        }
                    }
                });
                drop(join);
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

pub struct Ticker<C> {
    rx: Receiver<(C, Option<Timespec>)>,
    tx: Sender<(C, Option<Timespec>)>,
    joiner: thread::JoinHandle<()>,
    leave_guard: Arc<(Mutex<bool>, Condvar)>,
}

impl<C> Ticker<C> where C: Send + Copy + 'static {
    pub fn spawn(server: &str, tick_interval: Duration, ntp_poll: Duration, event: C) -> Ticker<C> {
        let (tx, rx) = channel();
        let leave_guard = Arc::new((Mutex::new(true), Condvar::new()));
        let waiter = leave_guard.clone();
        let cloned_tx = tx.clone();
        let server = server.into();

        let joiner = thread::spawn(move || {
            let mut ntp = NtpFetcher::new(server, ntp_poll, waiter.clone());
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

    pub fn stop_ticker(self) {
        let &(ref lock, ref cvar) = &*(self.leave_guard);
        let mut leaver = lock.lock().ok().expect("BUG: cannot claim mutex during stop_ticker");
        *leaver = false;
        drop(leaver);
        cvar.notify_all();
        let _ = self.joiner.join();
    }

    pub fn get_sender(&self) -> Sender<(C, Option<Timespec>)> {
        self.tx.clone()
    }

    pub fn recv_iter(&self) -> Iter<(C, Option<Timespec>)> {
        self.rx.iter()
    }
}
