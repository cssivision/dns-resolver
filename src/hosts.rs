use std::collections::HashMap;
use std::fs;
use std::time::{Duration, SystemTime};
use std::io::{BufRead, BufReader};
use std::fs::File;
use std::net::{Ipv4Addr, Ipv6Addr};

lazy_static! {
    static ref CACHE_MAX_AGE: Duration = Duration::new(5, 0);
}

#[derive(Debug)]
pub struct Hosts {
    by_name: HashMap<String, Vec<String>>,
    by_addr: HashMap<String, Vec<String>>,
    expire: SystemTime,
    path: String,
    mtime: SystemTime,
    size: u64,
}

impl Hosts {
    pub fn new() -> Hosts {
        let path = get_path();
        let mut hosts = Hosts {
            by_name: HashMap::new(),
            by_addr: HashMap::new(),
            expire: SystemTime::now() + *CACHE_MAX_AGE,
            path: path.clone(),
            mtime: SystemTime::now(),
            size: 0,
        };

        hosts.update();
        hosts
    }

    fn update(&mut self) {
        let now = SystemTime::now();
        if now < self.expire && self.path == get_path() && self.by_name.len() > 0 {
            return;
        }

        self.path = get_path();
        let meta = if let Ok(meta) = fs::metadata(&self.path) {
            meta
        } else {
            error!("update fail, {} not found", self.path);
            return;
        };

        if self.path == get_path() && self.mtime == meta.modified().unwrap_or(SystemTime::now()) &&
            self.size == meta.len()
        {
            self.expire = now + *CACHE_MAX_AGE;
            return;
        }

        let f = if let Ok(f) = File::open(&self.path) {
            f
        } else {
            error!("update fail, open {} fail", self.path);
            return;
        };

        for line in BufReader::new(f).lines() {
            let line = line.unwrap_or_default();
            let line = if let Some(pos) = line.find('#') {
                String::from(line.split_at(pos).0)
            } else {
                line
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let fields: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
            if fields.len() < 2 {
                continue;
            }
            let addr = if let Some(a) = parse_literal_ip(fields[0].clone()) {
                a
            } else {
                continue;
            };

            for i in 1..fields.len() {
                let key = fields[i].clone();

                if self.by_addr.get_mut(&addr).is_none() {
                    self.by_addr.insert(addr.clone(), Vec::new());
                }
                if let Some(val) = self.by_addr.get_mut(&addr) {
                    val.push(key.clone());
                }

                if self.by_name.get_mut(&key).is_none() {
                    self.by_name.insert(key.clone(), Vec::new());
                }
                if let Some(val) = self.by_name.get_mut(&key) {
                    val.push(addr.clone());
                }
            }
        }

        self.mtime = meta.modified().unwrap_or(SystemTime::now());
        self.size = meta.len();
        self.path = get_path();
        self.expire = SystemTime::now() + *CACHE_MAX_AGE;
    }
}

fn get_path() -> String {
    "/etc/hosts".to_string()
}

fn parse_literal_ip(addr: String) -> Option<String> {
    let ip4 = addr.parse::<Ipv4Addr>();
    let ip6 = addr.parse::<Ipv6Addr>();
    if ip4.is_ok() || ip6.is_ok() {
        return Some(addr);
    }
    None
}
