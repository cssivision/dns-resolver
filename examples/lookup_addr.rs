extern crate dns_resolver;

fn main() {
    dns_resolver::lookup_host(String::from("www.baidu.com"));
}
