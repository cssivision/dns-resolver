use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;

const DEFAULT_PORT: u16 = 53;

pub fn read_system_conf() -> io::Result<()> {
    read_resolv_conf("/etc/resolv.conf")
}

fn read_resolv_conf<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let mut data = String::new();
    let mut file = File::open(path)?;
    file.read_to_string(&mut data)?;
    let parsed_conf = resolv_conf::Config::parse(&data).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Error parsing resolv.conf: {:?}", e),
        )
    })?;
    Ok(())
}
