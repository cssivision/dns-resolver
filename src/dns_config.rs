//! Read system DNS config from /etc/resolv.conf

use std::time::{Duration, SystemTime};
use std::io;
use std::fs::File;
use std::io::{BufRead, BufReader};

use hostname::get_hostname;
use hosts::parse_literal_ip;

lazy_static! {
    static ref DEFAULT_DNS: Vec<String> = vec!["127.0.0.1:53".to_string(), "[::1]:53".to_string()];
}

static BIG: i32 = 0xFFFFFF;

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
            conf.servers = DEFAULT_DNS.clone();
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

        let fields: Vec<&str> = line.split_whitespace().collect();
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
                    s if s.starts_with("ndots:") => {
                        if let Some(s) = s.get(6..) {
                            let mut n = dtoi(s).0;
                            if n < 0 {
                                n = 0;
                            } else if n > 15 {
                                n = 15;
                            }
                            conf.ndots = n;
                        } else {
                            debug!("invalid ndots");
                        };
                    }
                    s if s.starts_with("timeout:") => {
                        if let Some(s) = s.get(8..) {
                            let mut n = dtoi(s).0;
                            if n < 1 {
                                n = 1;
                            }
                            conf.timeout = Duration::new(n as u64, 0);
                        } else {
                            debug!("invalid timeout");
                        };
                    }
                    s if s.starts_with("attempts:") => {
                        if let Some(s) = s.get(9..) {
                            let mut n = dtoi(s).0;
                            if n < 1 {
                                n = 1;
                            }
                            conf.attempts = n;
                        } else {
                            debug!("invalid attempts");
                        };
                    }
                    "rotate" => {
                        conf.rotate = true;
                    }
                    _ => {
                        conf.unknown_opt = true;
                    }
                }
            },
            "lookup" => {
                conf.lookup = fields[1..].iter().map(|x| x.to_string()).collect();
            }
            _ => {
                conf.unknown_opt = true;
            }
        }
    }

    if conf.servers.is_empty() {
        conf.servers = DEFAULT_DNS.clone();
    }

    if conf.search.is_empty() {
        conf.search = default_search();
    }

    conf
}

fn dtoi(s: &str) -> (i32, i32, bool) {
    let mut i: i32 = 0;
    let mut n: i32 = 0;
    let chars = s.chars();
    for c in chars {
        if c < '0' || c > '9' {
            break;
        }
        n = n * 10 + (c as u8 - '0' as u8) as i32;
        if n >= BIG {
            return (BIG, i, false);
        }
        i = i + 1;
    }

    if i == 0 {
        return (0, 0, false);
    }

    (n, i, true)
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
