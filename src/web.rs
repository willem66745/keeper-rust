use iron::status;
use iron::prelude::*;
use iron::mime::Mime;
use router::Router;
use super::tracker::{TrackerClient, Context};
use rustc_serialize::json;
use std::collections::BTreeMap;
use time::at_utc;

pub struct Web;

impl Web {
    pub fn new() -> Web {
        Web
    }

    pub fn serve(&mut self, tracker: TrackerClient) {
        let mut router = Router::new();

        // JSON: get available switches
        let tracker4switch = tracker.clone();
        router.get("/switches", move|_: &mut Request| {
            let switches = tracker4switch.get_list();
            let content_type = "application/json".parse::<Mime>().unwrap();
            Ok(Response::with((content_type, status::Ok, format!("{}", json::as_json(&switches)))))
        });

        // JSON: retrieve switch status
        let tracker4get = tracker.clone();
        router.get("/get/:switch", move|req: &mut Request| {
            let ref switch = req.extensions.get::<Router>().unwrap().find("switch");
            let content_type = "application/json".parse::<Mime>().unwrap();

            #[derive(RustcEncodable)]
            struct GetResult {
                switch: bool,
                next_events: BTreeMap<String, bool>
            }

            Ok(switch.and_then(|ref switch| tracker4get.get_switch(&switch)).map_or(
                Response::with(status::NotFound), |(ref now, ref next)| {
                    let mut next_events = BTreeMap::new();

                    for (ts, state) in next {
                        let _ = next_events.insert(
                            format!("{}", at_utc(*ts).rfc3339()),
                            *state == Context::On);
                    }

                    let get_result = GetResult {
                        switch: *now == Context::On,
                        next_events: next_events,
                    };
                    let json = json::as_json(&get_result);
                    Response::with((content_type, status::Ok, format!("{}", json)))
                }))
        });

        Iron::new(router).http("0.0.0.0:3000").unwrap();
    }
}
