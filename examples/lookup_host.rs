extern crate dns_resolver;

fn main() {
    dns_resolver::lookup_host("www.baidu.com");
}
