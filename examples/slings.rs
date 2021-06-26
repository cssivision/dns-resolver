use std::io;

use dns_resolver::Resolver;
use slings::runtime::Runtime;

#[cfg(feature = "slings-runtime")]
fn main() -> io::Result<()> {
    let runtime = Runtime::new()?;
    let resolver = Resolver::new();

    runtime.block_on(async {
        let ips = resolver.lookup_host("baidu.com").await?;
        println!("ips: {:?}", ips);
        Ok(())
    })
}

#[cfg(not(feature = "slings-runtime"))]
fn main() {
    println!("slings-runtime feature must be enabled")
}
