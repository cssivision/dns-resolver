use std::time::{Duration, SystemTime};
use std::io;
use std::fs::File;
use std::io::prelude::*;

use hostname::get_hostname;

lazy_static! {
    static ref DEFAULT_NS: Vec<String> = vec!["127.0.0.1:53".to_string(), "[::1]:53".to_string()];
}

#[derive(Debug, Default)]
pub struct DnsConfig {
    servers: Vec<String>,      // server addresses (in host:port form) to use
    search: Vec<String>,       // rooted suffixes to append to local name
    ndots: i32,                // number of dots in name to trigger absolute lookup
    timeout: Duration,         // wait before giving up on a query, including retries
    attempts: i32,             // lost packets before giving up on server
    rotate: bool,              // round robin among servers
    unknown_opt: bool,         // anything unknown was encountered
    lookup: Vec<String>,       // OpenBSD top-level database "lookup" order
    err: Option<io::Error>,    // any error that occurs during open of resolv.conf
    mtime: Option<SystemTime>, // time of resolv.conf modification
    soffset: u32,              // used by serverOffset
}

pub fn read_config(filename: &str) -> DnsConfig {
    let mut conf = DnsConfig {
        ndots: 1,
        timeout: Duration::new(5, 0),
        attempts: 2,
        ..Default::default()
    };

    let file = match File::open(filename) {
        Ok(f) => f,
        Err(e) => {
            conf.servers = DEFAULT_NS.clone();
            conf.servers = default_search();
            conf.err = Some(e);
            return conf;
        }
    };

    unimplemented!()
}

fn default_search() -> Vec<String> {
    let hostname = if let Ok(hs) = get_hostname() {
        hs
    } else {
        return Vec::new();
    };

    if let Some(pos) = hostname.find('.') {
        return vec![hostname[pos + 1..].to_string()];
    };

    Vec::new()
}
