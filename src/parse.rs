use std::net::{Ipv4Addr, Ipv6Addr};

pub fn parse_literal_ip(addr: &str) -> Option<String> {
    let ip4 = addr.parse::<Ipv4Addr>();
    let ip6 = addr.parse::<Ipv6Addr>();
    if ip4.is_ok() || ip6.is_ok() {
        return Some(addr.to_string());
    }
    None
}
