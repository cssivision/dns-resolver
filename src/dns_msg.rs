#[derive(Default, Debug)]
pub struct DnsMsgHeader {
    id: u16,
    response: bool,
    opcode: i32,
    authoritative: bool,
    truncated: bool,
    recursion_desired: bool,
    recursion_available: bool,
    rcode: i32,
}

#[derive(Default, Debug)]
struct DnsQuestion {
    name: String,
    q_type: u16,
    q_class: u16,
}

struct DnsMsg {}
