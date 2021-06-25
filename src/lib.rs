use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

mod system_conf;

use dnssector::{
    Class, DNSSector, ParsedPacket, RdataIterable, Type, DNS_FLAG_QR, DNS_FLAG_TC,
    DNS_MAX_COMPRESSED_SIZE,
};
use slings::net::{TcpStream, UdpSocket};
use slings::time::timeout;
use slings::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct Resolver {
    servers: Vec<SocketAddr>,
    timeout: Duration,
}

fn invalid_input(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg)
}

impl Resolver {
    pub fn new(addr: SocketAddr) -> Self {
        Resolver {
            servers: vec![addr],
            timeout: Duration::from_secs(3),
        }
    }

    async fn query_upstream(
        &self,
        addr: &SocketAddr,
        tid: u16,
        question: &Option<(Vec<u8>, u16, u16)>,
        query: &[u8],
    ) -> io::Result<ParsedPacket> {
        let mut res = timeout(self.timeout, query_upstream_udp(query, &addr)).await??;
        if res.flags() & DNS_FLAG_TC == DNS_FLAG_TC {
            res = timeout(self.timeout, query_upstream_tcp(query, &addr)).await??;
        }
        if res.tid() != tid || &res.question() != question {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Unexpected response",
            ));
        }
        Ok(res)
    }

    async fn query_packet(&self, mut query: ParsedPacket) -> io::Result<ParsedPacket> {
        let tid = query.tid();
        let question = query.question();
        if question.is_none() || query.flags() & DNS_FLAG_QR != 0 {
            return Err(invalid_input("No DNS question"));
        }
        let query = query.into_packet();
        for server in &self.servers {
            if let Ok(res) = self.query_upstream(&server, tid, &question, &query).await {
                return Ok(res);
            }
        }
        Err(invalid_input("No response received from any servers"))
    }

    pub async fn query_a(&self, name: &str) -> io::Result<Vec<Ipv4Addr>> {
        let query = dnssector::gen::query(name.as_bytes(), Type::A, Class::IN)
            .map_err(|e| invalid_input(&e.to_string()))?;
        let mut res = self.query_packet(query).await?;
        let mut ips = vec![];
        for item in res.into_iter_answer().into_iter() {
            if let Ok(IpAddr::V4(addr)) = item.rr_ip() {
                ips.push(addr);
            }
        }
        Ok(ips)
    }

    pub async fn query_aaaa(&self, name: &str) -> io::Result<Vec<Ipv6Addr>> {
        let query = dnssector::gen::query(name.as_bytes(), Type::AAAA, Class::IN)
            .map_err(|e| invalid_input(&e.to_string()))?;
        let mut res = self.query_packet(query).await?;
        let mut ips = vec![];
        for item in res.into_iter_answer().into_iter() {
            if let Ok(IpAddr::V6(addr)) = item.rr_ip() {
                ips.push(addr);
            }
        }
        Ok(ips)
    }
}

async fn query_upstream_udp(query: &[u8], addr: &SocketAddr) -> io::Result<ParsedPacket> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(addr)?;
    socket.send(&query).await?;
    let mut response = vec![0; DNS_MAX_COMPRESSED_SIZE];
    let n = socket
        .recv(&mut response)
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::WouldBlock, "Timeout"))?;
    response.truncate(n);
    DNSSector::new(response)
        .map_err(|e| invalid_input(&e.to_string()))?
        .parse()
        .map_err(|e| invalid_input(&e.to_string()))
}

async fn query_upstream_tcp(query: &[u8], addr: &SocketAddr) -> io::Result<ParsedPacket> {
    let mut stream = TcpStream::connect(addr).await?;
    let _ = stream.set_nodelay(true);
    let query_len = query.len();
    let mut tcp_query = Vec::with_capacity(2 + query_len);
    tcp_query.push((query_len >> 8) as u8);
    tcp_query.push(query_len as u8);
    tcp_query.extend_from_slice(query);
    stream.write_all(&tcp_query).await?;
    let mut response_len_bytes = [0u8; 2];
    stream.read_exact(&mut response_len_bytes).await?;
    let response_len = ((response_len_bytes[0] as usize) << 8) | (response_len_bytes[1] as usize);
    let mut response = vec![0; response_len];
    stream.read_exact(&mut response).await?;
    DNSSector::new(response)
        .map_err(|e| invalid_input(&e.to_string()))?
        .parse()
        .map_err(|e| invalid_input(&e.to_string()))
}
