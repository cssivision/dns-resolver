use std::time::{Duration, SystemTime};
use std::io;

lazy_static! {
    static ref DEFAULT_DNS: Vec<String> = vec!["127.0.0.1:53".to_string(), "[::1]:53".to_string()];
}

#[derive(Debug)]
struct DnsConfig {
    server: Vec<String>, // server addresses (in host:port form) to use
    search: Vec<String>, // rooted suffixes to append to local name
    ndots: i32,          // number of dots in name to trigger absolute lookup
    timeout: Duration,   // wait before giving up on a query, including retries
    attempts: i32,       // lost packets before giving up on server
    rotate: bool,        // round robin among servers
    unknownOpt: bool,    // anything unknown was encountered
    lookup: Vec<String>, // OpenBSD top-level database "lookup" order
    err: io::Error,      // any error that occurs during open of resolv.conf
    mtime: SystemTime,   // time of resolv.conf modification
    soffset: u32,        // used by serverOffset
}
