
pub static DNS_TYPEA: u32 = 1;
pub static DNS_TYPEAAAA: u32 = 28;
pub static DNS_ClASSINET: u32 = 1;

#[derive(Default, Debug)]
pub struct DnsMsgHeader {
    pub id: u16,
    pub response: bool,
    pub opcode: i32,
    pub authoritative: bool,
    pub truncated: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    pub rcode: i32,
}

#[derive(Default, Debug)]
pub struct DnsQuestion {
    pub name: String,
    pub qtype: u16,
    pub qclass: u16,
}

#[derive(Debug)]
pub struct DnsMsg {
    pub header: DnsMsgHeader,
    pub question: Vec<DnsQuestion>,
}
