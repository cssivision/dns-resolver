use std::io;
use std::fs;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use std::net::SocketAddr;

use hosts::lookup_static_host;
use dns_config::{read_config, DnsConfig};
use dns_msg::{DNS_ClASSINET, DnsMsg, DnsMsgHeader, DnsQuestion, DNS_TYPEA, DNS_TYPEAAAA};

use rand::{self, Rng};
use tokio_core::net::{TcpStream, UdpSocket};
use tokio_core::reactor::{Core, Remote};
use tokio_timer::Timer;
use futures::Future;

lazy_static! {
    static ref CACHE_MAX_AGE: Duration = Duration::new(5, 0);
    pub static ref DEFAULT_RESOLVER: Resolver = Resolver::new();
    static ref RESOLV_CONF: Mutex<ResolverConfig> = Mutex::new(ResolverConfig::new());
}

trait DnsConn {
    fn request(conf: &DnsConfig) -> Result<&DnsConfig, io::Error>;
}

#[derive(Debug)]
struct DnsPacket {}

impl DnsConn for DnsPacket {
    fn request(conf: &DnsConfig) -> Result<&DnsConfig, io::Error> {
        unimplemented!()
    }
}

#[derive(Debug)]
struct DnsStream {}

impl DnsConn for DnsStream {
    fn request(conf: &DnsConfig) -> Result<&DnsConfig, io::Error> {
        unimplemented!()
    }
}

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
pub struct Resolver {
    remote: Remote,
}

impl Resolver {
    pub fn new() -> Resolver {
        let core = Core::new().unwrap();

        Resolver {
            remote: core.remote(),
        }
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
                self.exchange(&server, name, qtype, conf.timeout);
            }
        }
    }

    fn exchange(&self, server: &str, name: &str, qtype: u32, timeout: Duration) {
        let mut out = DnsMsg {
            header: DnsMsgHeader {
                recursion_available: true,
                ..Default::default()
            },
            question: vec![
                DnsQuestion {
                    name: name.to_string(),
                    qtype: qtype as u16,
                    qclass: DNS_ClASSINET as u16,
                },
            ],
        };

        let handle = self.remote.handle().unwrap();
        let timer = Timer::default();
        let addr = server.parse().unwrap();
        let udp_conn = UdpSocket::bind(&"0.0.0.0:0".parse::<SocketAddr>().unwrap(), &handle);

        let mut rng = rand::thread_rng();
        out.header.id = rng.gen::<u16>();
    }
}

fn other(desc: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, desc)
}
