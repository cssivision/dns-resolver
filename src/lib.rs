#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

mod hosts;
use hosts::Hosts;

pub fn lookup_host(host: String) {
    let h = Hosts::new();
    println!("{}", host);
    println!("{:?}", h);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        lookup_host("www.baidu.com".to_string());
    }
}
