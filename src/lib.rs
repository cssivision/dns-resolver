#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

mod hosts;

pub fn lookup_host(host: String) {
    let hosts = hosts::HOSTS.lock().unwrap();
    println!("{:?}", hosts);
    println!("{}", host);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        lookup_host("www.baidu.com".to_string());
    }
}
