use std::io;

use dns_resolver::Resolver;
use slings::runtime::Runtime;

fn main() -> io::Result<()> {
    let runtime = Runtime::new()?;
    runtime.block_on(async {
        let resolver = Resolver::new();
        let ips = resolver.lookup_host("baidu.com").await?;
        println!("ips: {:?}", ips);
        Ok(())
    })
}
