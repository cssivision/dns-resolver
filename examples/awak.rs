use dns_resolver::Resolver;
use std::io;

#[cfg(feature = "awak-runtime")]
fn main() -> io::Result<()> {
    let resolver = Resolver::new();

    awak::block_on(async {
        let ips = resolver.lookup_host("baidu.com").await?;
        println!("ips: {:?}", ips);
        Ok(())
    })
}

#[cfg(not(feature = "awak-runtime"))]
fn main() {
    println!("awak-runtime feature must be enabled")
}
