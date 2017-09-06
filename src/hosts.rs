use std::collections::HashMap;
use std::fs;
use std::ops::Add;
use std::time::{Duration, SystemTime};

lazy_static! {
    pub static ref HOSTS: Hosts = Hosts::new();
    static ref CACHE_MAX_AGE: Duration = Duration::new(5, 0);
}

#[derive(Debug)]
pub struct Hosts {
    by_name: HashMap<String, String>,
    by_addr: HashMap<String, String>,
    expire: SystemTime,
    path: String,
    mtime: SystemTime,
    size: u64,
}

impl Hosts {
    fn new() -> Hosts {
        let path = get_path();
        let mut hosts = Hosts {
            by_name: HashMap::new(),
            by_addr: HashMap::new(),
            expire: SystemTime::now().add(*CACHE_MAX_AGE),
            path: path.clone(),
            mtime: SystemTime::now(),
            size: 0,
        };

        let meta = if let Ok(meta) = fs::metadata(&path) {
            meta
        } else {
            return hosts;
        };
        hosts.mtime = meta.modified().unwrap_or(SystemTime::now());
        hosts.size = meta.len();
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
            return;
        };

        if self.path == get_path() && self.mtime == meta.modified().unwrap_or(SystemTime::now()) &&
            self.size == meta.len()
        {
            self.expire = now.add(*CACHE_MAX_AGE);
            return;
        }
    }
}

fn get_path() -> String {
    "/etc/hosts".to_string()
}
