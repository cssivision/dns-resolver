#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

mod hosts;

pub fn lookup_host(host: &str) {
    let addrs = hosts::lookup_static_host(&host).unwrap();
    println!("{:?}", addrs);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        lookup_host("localhost");
    }
}
