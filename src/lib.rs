use std::io;
use std::net::{IpAddr, SocketAddr};
use std::ops::Deref;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use domain::base::iana::{Rcode, Rtype};
use domain::base::message::Message;
use domain::base::message_builder::{AdditionalBuilder, MessageBuilder, StreamTarget};
use domain::base::name::{Dname, ToDname};
use domain::base::octets::Octets512;
use domain::base::question::Question;
use domain::rdata::A;

#[cfg(feature = "slings-runtime")]
use slings::{
    net::{TcpStream, UdpSocket},
    time::timeout,
    AsyncReadExt, AsyncWriteExt,
};

#[cfg(feature = "awak-runtime")]
use awak::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
    time::timeout,
};

mod conf;

pub use conf::{ResolvConf, ResolvOptions};
use conf::{ServerConf, Transport};

const RETRY_RANDOM_PORT: usize = 10;

#[derive(Clone, Debug)]
pub struct Resolver {
    preferred: ServerList,
    stream: ServerList,
    options: ResolvOptions,
}

impl Resolver {
    pub fn new() -> Self {
        Self::from_conf(ResolvConf::default())
    }

    pub fn from_conf(conf: ResolvConf) -> Self {
        Resolver {
            preferred: ServerList::from_conf(&conf, |s| s.transport.is_preferred()),
            stream: ServerList::from_conf(&conf, |s| s.transport.is_stream()),
            options: conf.options,
        }
    }

    fn options(&self) -> &ResolvOptions {
        &self.options
    }

    pub async fn query<N: ToDname, Q: Into<Question<N>>>(&self, question: Q) -> io::Result<Answer> {
        Query::new(self)?
            .run(Query::create_message(question.into()))
            .await
    }

    pub async fn lookup_host<T: AsRef<str>>(&self, host: T) -> io::Result<Vec<IpAddr>> {
        let qname = &Dname::<Vec<u8>>::from_str(&host.as_ref())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let answer = self.query((&qname, Rtype::A)).await?;
        let name = answer.canonical_name();
        let mut records = answer
            .answer()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
            .limit_to::<A>();

        let mut ips = vec![];
        while let Some(res) = records.next() {
            if let Ok(record) = res {
                if Some(*record.owner()) == name {
                    ips.push(record.data().addr().into());
                }
            }
        }
        Ok(ips)
    }

    pub async fn query_message(&self, message: QueryMessage) -> io::Result<Answer> {
        Query::new(self)?.run(message).await
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Query<'a> {
    resolver: &'a Resolver,
    preferred: bool,
    attempt: usize,
    counter: ServerListCounter,
    error: io::Result<Answer>,
}

impl<'a> Query<'a> {
    pub fn new(resolver: &'a Resolver) -> io::Result<Self> {
        let (preferred, counter) = if resolver.options().use_vc || resolver.preferred.is_empty() {
            if resolver.stream.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "no servers available",
                ));
            }
            (false, resolver.stream.counter(resolver.options().rotate))
        } else {
            (true, resolver.preferred.counter(resolver.options().rotate))
        };
        Ok(Query {
            resolver,
            preferred,
            attempt: 0,
            counter,
            error: Err(io::Error::new(io::ErrorKind::TimedOut, "all timed out")),
        })
    }

    pub async fn run(mut self, mut message: QueryMessage) -> io::Result<Answer> {
        loop {
            match self.run_query(&mut message).await {
                Ok(answer) => {
                    if answer.header().rcode() == Rcode::FormErr
                        && self.current_server().does_edns()
                    {
                        self.current_server().disable_edns();
                        continue;
                    } else if answer.header().rcode() == Rcode::ServFail {
                        self.update_error_servfail(answer);
                    } else if answer.header().tc()
                        && self.preferred
                        && !self.resolver.options().ign_tc
                    {
                        if self.switch_to_stream() {
                            continue;
                        } else {
                            return Ok(answer);
                        }
                    } else {
                        return Ok(answer);
                    }
                }
                Err(err) => self.update_error(err),
            }
            if !self.next_server() {
                return self.error;
            }
        }
    }

    fn create_message(question: Question<impl ToDname>) -> QueryMessage {
        let mut message =
            MessageBuilder::from_target(StreamTarget::new(Octets512::new()).unwrap()).unwrap();
        message.header_mut().set_rd(true);
        let mut message = message.question();
        message.push(question).unwrap();
        message.additional()
    }

    async fn run_query(&mut self, message: &mut QueryMessage) -> io::Result<Answer> {
        let server = self.current_server();
        server.prepare_message(message);
        server.query(message).await
    }

    fn current_server(&self) -> &ServerInfo {
        let list = if self.preferred {
            &self.resolver.preferred
        } else {
            &self.resolver.stream
        };
        self.counter.info(list)
    }

    fn update_error(&mut self, err: io::Error) {
        if err.kind() != io::ErrorKind::TimedOut && self.error.is_err() {
            self.error = Err(err)
        }
    }

    fn update_error_servfail(&mut self, answer: Answer) {
        self.error = Ok(answer)
    }

    fn switch_to_stream(&mut self) -> bool {
        if !self.preferred {
            return false;
        }
        self.preferred = false;
        self.attempt = 0;
        self.counter = self.resolver.stream.counter(self.resolver.options().rotate);
        true
    }

    fn next_server(&mut self) -> bool {
        if self.counter.next() {
            return true;
        }
        self.attempt += 1;
        if self.attempt >= self.resolver.options().attempts {
            return false;
        }
        self.counter = if self.preferred {
            self.resolver
                .preferred
                .counter(self.resolver.options().rotate)
        } else {
            self.resolver.stream.counter(self.resolver.options().rotate)
        };
        true
    }
}

pub type QueryMessage = AdditionalBuilder<StreamTarget<Octets512>>;

#[derive(Clone)]
pub struct Answer {
    message: Message<Vec<u8>>,
}

impl Answer {
    pub fn is_final(&self) -> bool {
        (self.message.header().rcode() == Rcode::NoError
            || self.message.header().rcode() == Rcode::NXDomain)
            && !self.message.header().tc()
    }

    pub fn is_truncated(&self) -> bool {
        self.message.header().tc()
    }

    pub fn into_message(self) -> Message<Vec<u8>> {
        self.message
    }
}

impl From<Message<Vec<u8>>> for Answer {
    fn from(message: Message<Vec<u8>>) -> Self {
        Answer { message }
    }
}

#[derive(Clone, Debug)]
struct ServerInfo {
    conf: ServerConf,
    edns: Arc<AtomicBool>,
}

impl ServerInfo {
    pub fn does_edns(&self) -> bool {
        self.edns.load(Ordering::Relaxed)
    }

    pub fn disable_edns(&self) {
        self.edns.store(false, Ordering::Relaxed);
    }

    pub fn prepare_message(&self, query: &mut QueryMessage) {
        query.rewind();
        if self.does_edns() {
            query
                .opt(|opt| {
                    opt.set_udp_payload_size(self.conf.udp_payload_size);
                    Ok(())
                })
                .unwrap();
        }
    }

    pub async fn query(&self, query: &QueryMessage) -> io::Result<Answer> {
        let res = match self.conf.transport {
            Transport::Udp => {
                timeout(
                    self.conf.request_timeout,
                    Self::udp_query(query, self.conf.addr, self.conf.recv_size),
                )
                .await
            }
            Transport::Tcp => {
                timeout(
                    self.conf.request_timeout,
                    Self::tcp_query(query, self.conf.addr),
                )
                .await
            }
        };
        match res {
            Ok(Ok(answer)) => Ok(answer),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "request timed out")),
        }
    }

    pub async fn tcp_query(query: &QueryMessage, addr: SocketAddr) -> io::Result<Answer> {
        let sock = &mut TcpStream::connect(&addr).await?;
        sock.write_all(query.as_target().as_stream_slice()).await?;

        loop {
            let mut len_buf = [0u8; 2];
            println!("len: {}", len_buf.len());
            sock.read_exact(&mut len_buf).await?;
            let len = u16::from_be_bytes(len_buf) as u64;
            let mut buf = Vec::new();
            sock.take(len).read_to_end(&mut buf).await?;
            if let Ok(answer) = Message::from_octets(buf.into()) {
                if answer.is_answer(&query.as_message()) {
                    return Ok(answer.into());
                }
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, "short buf"));
            }
        }
    }

    pub async fn udp_query(
        query: &QueryMessage,
        addr: SocketAddr,
        recv_size: usize,
    ) -> io::Result<Answer> {
        let sock = Self::udp_bind(addr.is_ipv4()).await?;
        sock.connect(addr)?;
        let sent = sock.send(query.as_target().as_dgram_slice()).await?;
        if sent != query.as_target().as_dgram_slice().len() {
            return Err(io::Error::new(io::ErrorKind::Other, "short UDP send"));
        }
        loop {
            let mut buf = vec![0; recv_size];
            let len = sock.recv(&mut buf).await?;
            buf.truncate(len);
            let answer = match Message::from_octets(buf.into()) {
                Ok(answer) => answer,
                Err(_) => continue,
            };
            if !answer.is_answer(&query.as_message()) {
                continue;
            }
            return Ok(answer.into());
        }
    }

    async fn udp_bind(v4: bool) -> io::Result<UdpSocket> {
        let mut i = 0;
        loop {
            let local: SocketAddr = if v4 {
                ([0u8; 4], 0).into()
            } else {
                ([0u16; 8], 0).into()
            };
            match UdpSocket::bind(&local) {
                Ok(sock) => return Ok(sock),
                Err(err) => {
                    if i == RETRY_RANDOM_PORT {
                        return Err(err);
                    } else {
                        i += 1
                    }
                }
            }
        }
    }
}

impl From<ServerConf> for ServerInfo {
    fn from(conf: ServerConf) -> Self {
        ServerInfo {
            conf,
            edns: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl<'a> From<&'a ServerConf> for ServerInfo {
    fn from(conf: &'a ServerConf) -> Self {
        conf.clone().into()
    }
}

#[derive(Clone, Debug)]
struct ServerList {
    servers: Vec<ServerInfo>,
    start: Arc<AtomicUsize>,
}

impl ServerList {
    pub fn from_conf<F>(conf: &ResolvConf, filter: F) -> Self
    where
        F: Fn(&ServerConf) -> bool,
    {
        ServerList {
            servers: {
                conf.servers
                    .iter()
                    .filter(|f| filter(*f))
                    .map(Into::into)
                    .collect()
            },
            start: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }

    pub fn counter(&self, rotate: bool) -> ServerListCounter {
        let res = ServerListCounter::new(self);
        if rotate {
            self.rotate()
        }
        res
    }

    pub fn iter(&self) -> ServerListIter {
        ServerListIter::new(self)
    }

    pub fn rotate(&self) {
        self.start.fetch_add(1, Ordering::SeqCst);
    }
}

impl<'a> IntoIterator for &'a ServerList {
    type Item = &'a ServerInfo;
    type IntoIter = ServerListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Deref for ServerList {
    type Target = [ServerInfo];

    fn deref(&self) -> &Self::Target {
        self.servers.as_ref()
    }
}

#[derive(Clone, Debug)]
struct ServerListCounter {
    cur: usize,
    end: usize,
}

impl ServerListCounter {
    fn new(list: &ServerList) -> Self {
        if list.servers.is_empty() {
            return ServerListCounter { cur: 0, end: 0 };
        }

        let start = list.start.load(Ordering::Relaxed) % list.servers.len();
        ServerListCounter {
            cur: start,
            end: start + list.servers.len(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> bool {
        let next = self.cur + 1;
        if next < self.end {
            self.cur = next;
            true
        } else {
            false
        }
    }

    pub fn info<'a>(&self, list: &'a ServerList) -> &'a ServerInfo {
        &list[self.cur % list.servers.len()]
    }
}

#[derive(Clone, Debug)]
struct ServerListIter<'a> {
    servers: &'a ServerList,
    counter: ServerListCounter,
}

impl<'a> ServerListIter<'a> {
    fn new(list: &'a ServerList) -> Self {
        ServerListIter {
            servers: list,
            counter: ServerListCounter::new(list),
        }
    }
}

impl<'a> Iterator for ServerListIter<'a> {
    type Item = &'a ServerInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter.next() {
            Some(self.counter.info(self.servers))
        } else {
            None
        }
    }
}

impl Deref for Answer {
    type Target = Message<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl AsRef<Message<Vec<u8>>> for Answer {
    fn as_ref(&self) -> &Message<Vec<u8>> {
        &self.message
    }
}
