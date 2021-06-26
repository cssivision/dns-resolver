use std::io;

use dns_resolver::Resolver;

#[cfg(feature = "slings-runtime")]
fn main() -> io::Result<()> {
    use slings::runtime::Runtime;
    let runtime = Runtime::new()?;
    runtime.block_on(async {
        let resolver = Resolver::new();
        let ips = resolver.lookup_host("baidu.com").await?;
        println!("ips: {:?}", ips);
        Ok(())
    })
}

#[cfg(not(feature = "slings-runtime"))]
fn main() {
    println!("slings-runtime feature must be enabled")
}
