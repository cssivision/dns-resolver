use std::fs::File;
use std::io::{self, Read};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use domain::base::name::Dname;

#[derive(Clone, Debug)]
pub struct ResolvOptions {
    pub search: SearchList,
    pub ndots: usize,
    pub timeout: Duration,
    pub attempts: usize,
    pub aa_only: bool,
    pub use_vc: bool,
    pub primary: bool,
    pub ign_tc: bool,
    pub use_inet6: bool,
    pub rotate: bool,
    pub no_check_names: bool,
    pub use_bstring: bool,
    pub use_ip6dotint: bool,
    pub use_edns0: bool,
    pub single_request: bool,
    pub single_request_reopen: bool,
    pub no_tld_query: bool,
}

impl Default for ResolvOptions {
    fn default() -> Self {
        ResolvOptions {
            search: SearchList::new(),
            ndots: 1,
            timeout: Duration::new(5, 0),
            attempts: 2,
            aa_only: false,
            use_vc: false,
            primary: false,
            ign_tc: false,
            use_inet6: false,
            rotate: false,
            no_check_names: false,
            use_bstring: false,
            use_ip6dotint: false,
            use_edns0: false,
            single_request: false,
            single_request_reopen: false,
            no_tld_query: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Transport {
    Udp,
    Tcp,
}

impl Transport {
    pub fn is_preferred(self) -> bool {
        match self {
            Transport::Udp => true,
            Transport::Tcp => false,
        }
    }

    pub fn is_stream(self) -> bool {
        match self {
            Transport::Udp => false,
            Transport::Tcp => true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ServerConf {
    pub addr: SocketAddr,
    pub transport: Transport,
    pub request_timeout: Duration,
    pub recv_size: usize,
    pub udp_payload_size: u16,
}

impl ServerConf {
    pub fn new(addr: SocketAddr, transport: Transport) -> Self {
        ServerConf {
            addr,
            transport,
            request_timeout: Duration::from_secs(2),
            recv_size: 1232,
            udp_payload_size: 1232,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResolvConf {
    pub servers: Vec<ServerConf>,
    pub options: ResolvOptions,
}

impl ResolvConf {
    pub fn new() -> Self {
        ResolvConf {
            servers: Vec::new(),
            options: ResolvOptions::default(),
        }
    }

    pub fn finalize(&mut self) {
        if self.servers.is_empty() {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 53);
            self.servers.push(ServerConf::new(addr, Transport::Udp));
            self.servers.push(ServerConf::new(addr, Transport::Tcp));
        }
        if self.options.search.is_empty() {
            self.options.search.push(Dname::root())
        }
        for server in &mut self.servers {
            server.request_timeout = self.options.timeout
        }
    }

    pub fn default() -> Self {
        let mut res = ResolvConf::new();
        let _ = res.parse_file("/etc/resolv.conf");
        res.finalize();
        res
    }
}

fn parse_resolv_conf<T: AsRef<[u8]>>(data: T) -> io::Result<resolv_conf::Config> {
    resolv_conf::Config::parse(&data).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Error parsing resolv.conf: {:?}", e),
        )
    })
}

impl ResolvConf {
    pub fn parse_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let mut data = String::new();
        let mut file = File::open(path)?;
        file.read_to_string(&mut data)?;
        let parsed_config = parse_resolv_conf(&data)?;
        self.fill(parsed_config)
    }

    fn fill(&mut self, parsed_config: resolv_conf::Config) -> io::Result<()> {
        let domain = parsed_config.get_system_domain().unwrap_or_default();
        let domain = SearchSuffix::from_str(&domain).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Error parsing resolv.conf: {:?}", e),
            )
        })?;
        self.options.search = domain.into();

        for ip in parsed_config.get_nameservers_or_local() {
            let ip: IpAddr = ip.into();
            self.servers
                .push(ServerConf::new(SocketAddr::from((ip, 53)), Transport::Udp));
            self.servers
                .push(ServerConf::new(SocketAddr::from((ip, 53)), Transport::Tcp));
        }

        for search_domain in parsed_config.get_last_search_or_domain() {
            self.options
                .search
                .push(SearchSuffix::from_str(&search_domain).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("Error parsing resolv.conf: {:?}", e),
                    )
                })?);
        }

        self.options.ndots = parsed_config.ndots as usize;
        self.options.timeout = Duration::from_secs(parsed_config.timeout as u64);
        self.options.attempts = parsed_config.attempts as usize;
        self.options.rotate = parsed_config.rotate;
        self.options.no_check_names = parsed_config.no_check_names;
        self.options.use_inet6 = parsed_config.inet6;
        self.options.use_bstring = parsed_config.ip6_bytestring;
        self.options.use_ip6dotint = parsed_config.ip6_dotint;
        self.options.use_edns0 = parsed_config.edns0;
        self.options.single_request = parsed_config.single_request;
        self.options.single_request_reopen = parsed_config.single_request_reopen;
        self.options.no_tld_query = parsed_config.no_tld_query;
        self.options.use_vc = parsed_config.use_vc;
        Ok(())
    }
}

impl Default for ResolvConf {
    fn default() -> Self {
        Self::new()
    }
}

pub type SearchSuffix = Dname<Vec<u8>>;

#[derive(Clone, Debug, Default)]
pub struct SearchList {
    search: Vec<SearchSuffix>,
}

impl SearchList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, name: SearchSuffix) {
        self.search.push(name)
    }

    pub fn push_root(&mut self) {
        self.search.push(Dname::root())
    }

    pub fn get(&self, pos: usize) -> Option<&SearchSuffix> {
        self.search.get(pos)
    }

    pub fn as_slice(&self) -> &[SearchSuffix] {
        self.as_ref()
    }
}

impl From<SearchSuffix> for SearchList {
    fn from(name: SearchSuffix) -> Self {
        let mut res = Self::new();
        res.push(name);
        res
    }
}

impl AsRef<[SearchSuffix]> for SearchList {
    fn as_ref(&self) -> &[SearchSuffix] {
        self.search.as_ref()
    }
}

impl ops::Deref for SearchList {
    type Target = [SearchSuffix];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
