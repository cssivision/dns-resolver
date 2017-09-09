use std::io;
use std::time::{Duration, SystemTime};

use hosts::lookup_static_host;

lazy_static! {
    pub static ref DEFAULT_RESOLVER: Resolver = Resolver::new();
}

#[derive(Debug)]
pub struct Resolver {}

#[derive(Debug)]
struct ResolverConfig {
    last_checked: SystemTime,
}

impl Resolver {
    pub fn new() -> Resolver {
        Resolver {}
    }

    pub fn lookup_host(&self, host: &str) -> Result<Vec<String>, io::Error> {
        if let Some(addrs) = lookup_static_host(host) {
            if addrs.len() > 0 {
                return Ok(addrs);
            }
        };

        self.lookup_ip_cname(host);

        Err(other("not implement"))
    }

    fn lookup_ip_cname(&self, name: &str) -> Result<Vec<String>, io::Error> {
        unimplemented!()
    }
}

fn other(desc: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, desc)
}
