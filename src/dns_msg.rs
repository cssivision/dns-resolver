pub static DNS_TYPEA: u32 = 1;
pub static DNS_TYPEAAAA: u32 = 28;
pub static DNS_ClASSINET: u32 = 1;

// dnsHeader.Bits

// query/response (response=1)
static QR: u16 = 1 << 15;
// authoritative
static AA: u16 = 1 << 10;
// truncated
static TC: u16 = 1 << 9;
// recursion desired
static RD: u16 = 1 << 8;
// recursion available
static RA: u16 = 1 << 7;

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

// The wire format for the DNS packet header.
#[derive(Default, Debug)]
struct DnsHeader {
    id: u16,
    bits: u16,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
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

impl DnsMsg {
    fn pack(&self) -> Option<Vec<u8>> {
        let mut dh = DnsHeader {
            id: self.header.id,
            bits: (self.header.opcode << 11 | self.header.rcode) as u16,
            ..Default::default()
        };
        if self.header.recursion_available {
            dh.bits |= RA;
        }
        if self.header.recursion_desired {
            dh.bits |= RD;
        }
        if self.header.authoritative {
            dh.bits |= AA;
        }
        if self.header.response {
            dh.bits |= QR;
        }

        None
    }
}
