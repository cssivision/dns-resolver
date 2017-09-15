#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;

use std::io;

mod hosts;
mod resolver;
mod dns_config;
mod hostname;
mod parse;
mod dns_msg;

pub fn lookup_host(host: &str) -> Result<Vec<String>, io::Error> {
    if host.is_empty() {
        return Err(other(&format!("no such host: {}", host)));
    }
    if hosts::parse_literal_ip(host).is_some() {
        return Ok(vec![host.to_string()]);
    }

    if let Ok(addrs) = resolver::DEFAULT_RESOLVER.lookup_host(host) {
        return Ok(addrs);
    };
    unimplemented!()
}

fn other(desc: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, desc)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        lookup_host("localhost");
    }
}
