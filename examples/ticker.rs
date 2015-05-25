
extern crate keeper;
extern crate time;

use time::{at, Duration};
use keeper::ticker::Ticker;

#[derive(Copy, Clone)]
enum Dummy {
    Dummy
}

fn main() {
    let ticker = Ticker::spawn("nl.pool.ntp.org", Duration::seconds(1), Duration::days(1), Dummy::Dummy);

    let mut teller = 0; // XXX

    for (_, timestamp) in ticker.recv_iter() {
        if let Some(timestamp) = timestamp {
            println!("{}", at(timestamp).asctime());
        }

        teller = teller + 1;
        if teller >= 5 {
            break;
        }
    }

    ticker.stop_ticker();
}
