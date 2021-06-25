use std::io;

use dns_resolver::Resolver;
use slings::runtime::Runtime;

fn main() -> io::Result<()> {
    let runtime = Runtime::new()?;
    runtime.block_on(async {
        let resolver = Resolver::new("114.114.114.114:53".parse().unwrap());
        let ips = resolver.query_a("www.baidu.com").await?;
        println!("ips: {:?}", ips);
        Ok(())
    })
}
