use iron::status;
use iron::prelude::*;
use iron::mime::Mime;
use router::{Router};
use super::tracker::TrackerClient;
use rustc_serialize::json;

pub struct Web;

impl Web {
    pub fn new() -> Web {
        Web
    }

    pub fn serve(&mut self, tracker: TrackerClient) {
        let mut router = Router::new();

        let tracker4switch = tracker.clone();
        router.get("/switches", move|_: &mut Request| {
            let switches = tracker4switch.get_list();
            let content_type = "application/json".parse::<Mime>().unwrap();
            Ok(Response::with((content_type, status::Ok, format!("{}", json::as_json(&switches)))))
        });

        Iron::new(router).http("0.0.0.0:3000").unwrap();
    }
}
