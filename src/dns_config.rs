use std::time::{Duration, SystemTime};
use std::io;
use std::fs::File;
use std::io::{BufRead, BufReader};

use hostname::get_hostname;
use hosts::parse_literal_ip;

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

    let f = match File::open(filename) {
        Ok(f) => {
            conf.mtime = Some(if let Ok(meta) = f.metadata() {
                meta.modified().unwrap_or(SystemTime::now())
            } else {
                SystemTime::now()
            });
            f
        }
        Err(e) => {
            conf.servers = DEFAULT_NS.clone();
            conf.servers = default_search();
            conf.err = Some(e);
            return conf;
        }
    };

    for line in BufReader::new(f).lines() {
        let line = line.unwrap_or_default();
        if line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split_whitespace().map(|s| s).collect();
        if fields.len() < 1 {
            continue;
        }

        match fields[0] {
            "nameserver" => if fields.len() > 1 && conf.servers.len() < 3 {
                if let Some(_) = parse_literal_ip(fields[1]) {
                    conf.servers.push(join_host_port(fields[1], "53"));
                }
            },
            "domain" => if fields.len() > 1 && fields[1].len() > 0 {
                conf.servers.push(ensure_rooted(fields[1]));
            },
            "search" => for i in 1..fields.len() {
                conf.search.push(ensure_rooted(fields[i]));
            },
            "options" => for i in 1..fields.len() {
                match fields[i] {
                    s if s.starts_with("ndots:") => {}
                    s if s.starts_with("timeout:") => {}
                    s if s.starts_with("attempts:") => {}
                    "rotate" => {}
                    _ => {
                        conf.unknown_opt = true;
                    }
                }
            },
            "lookup" => {}
            _ => {}
        }
    }

    if conf.servers.is_empty() {
        conf.servers = DEFAULT_NS.clone();
    }

    if conf.search.is_empty() {
        conf.search = default_search();
    }

    conf
}

fn ensure_rooted(s: &str) -> String {
    if s.ends_with('.') {
        s.to_string()
    } else {
        let mut f = s.to_string();
        f.push('.');
        f
    }
}

fn join_host_port(host: &str, port: &str) -> String {
    if let Some(_) = host.find(':') {
        return format!("[{}]:{}", host, port);
    }

    format!("{}:{}", host, port)
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
