use std::io;

use dns_resolver::Resolver;

#[cfg(feature = "slings-runtime")]
fn main() -> io::Result<()> {
    let resolver = Resolver::new();

    slings::block_on(async {
        let ips = resolver.lookup_host("baidu.com").await?;
        println!("ips: {:?}", ips);
        Ok(())
    })
}

#[cfg(not(feature = "slings-runtime"))]
fn main() {
    println!("slings-runtime feature must be enabled")
}
