use std::io;
use std::fs;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use hosts::lookup_static_host;
use dns_config::{read_config, DnsConfig};

lazy_static! {
    static ref CACHE_MAX_AGE: Duration = Duration::new(5, 0);
    pub static ref DEFAULT_RESOLVER: Resolver = Resolver::new();
    static ref RESOLV_CONF: Mutex<ResolverConfig> = Mutex::new(ResolverConfig::new());
}

static DNS_TYPEA: u32 = 1;
static DNS_TYPEAAAA: u32 = 28;

#[derive(Debug)]
struct ResolverConfig {
    last_checked: SystemTime,
    dns_config: DnsConfig,
    path: String,
}

impl ResolverConfig {
    fn new() -> ResolverConfig {
        ResolverConfig {
            path: String::from("/etc/resolv.conf"),
            dns_config: read_config("/etc/resolv.conf"),
            last_checked: SystemTime::now() + *CACHE_MAX_AGE,
        }
    }

    fn update(&mut self) {
        let now = SystemTime::now();
        if self.last_checked > now {
            return;
        }

        self.last_checked = now + *CACHE_MAX_AGE;

        let meta = if let Ok(meta) = fs::metadata(&self.path) {
            meta
        } else {
            error!("update fail, {} not found", self.path);
            return;
        };

        if self.dns_config.mtime == meta.modified().map_err(|_| SystemTime::now()).ok() {
            return;
        }

        self.dns_config = read_config(&self.path);
    }
}

#[derive(Debug)]
pub struct Resolver {}

impl Resolver {
    pub fn new() -> Resolver {
        Resolver {}
    }

    pub fn lookup_host(&self, host: &str) -> Result<Vec<String>, io::Error> {
        if let Some(addrs) = lookup_static_host(host) {
            if addrs.len() > 0 {
                // sort ip addrs by RFC6724.
                // todo.
                return Ok(addrs);
            }
        };

        self.lookup_ip_cname(host);

        Err(other("not implement"))
    }

    fn lookup_ip_cname(&self, name: &str) -> Result<Vec<String>, io::Error> {
        let mut resolv_conf = RESOLV_CONF.lock().unwrap();
        resolv_conf.update();
        let conf = &resolv_conf.dns_config;
        let query_types = vec![DNS_TYPEA, DNS_TYPEAAAA];
        if let Some(list) = conf.name_list(name) {
            for i in 0..query_types.len() {
                self.try_one_name(conf, name, query_types[i])
            }
        };
        unimplemented!()
    }

    fn try_one_name(&self, conf: &DnsConfig, name: &str, qtype: u32) {
        let server_offset = conf.server_offset() as usize;

        let s_len = conf.servers.len();
        for i in 0..conf.attempts {
            for j in 0..s_len {
                let server = conf.servers[(server_offset + j) % s_len].clone();
            }
        }
    }

    fn exchange(&self, server: &str, name: &str, qtype: u32, timeout: Duration) {}

    fn dial(&self) {}
}

fn other(desc: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, desc)
}
