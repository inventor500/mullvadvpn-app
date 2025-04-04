//! This module keeps track of the last known good API IP address and reads and stores it on disk.

use crate::{ApiEndpoint, DnsResolver};
use async_trait::async_trait;
use std::{io, net::SocketAddr, path::Path, sync::Arc};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to open the address cache file")]
    Open(#[source] io::Error),

    #[error("Failed to read the address cache file")]
    Read(#[source] io::Error),

    #[error("Failed to parse the address cache file")]
    Parse,

    #[error("Failed to update the address cache file")]
    Write(#[source] io::Error),
}

/// A DNS resolver which resolves using `AddressCache`.
#[async_trait]
impl DnsResolver for AddressCache {
    async fn resolve(&self, host: String) -> Result<Vec<SocketAddr>, io::Error> {
        self.resolve_hostname(&host)
            .await
            .map(|addr| vec![addr])
            .ok_or(io::Error::other("host does not match API host"))
    }
}

#[derive(Clone)]
pub struct AddressCache {
    hostname: String,
    inner: Arc<Mutex<AddressCacheInner>>,
    write_path: Option<Arc<Path>>,
}

impl AddressCache {
    /// Initialize cache using the hardcoded address, and write changes to `write_path`.
    pub fn new(endpoint: &ApiEndpoint, write_path: Option<Box<Path>>) -> Self {
        Self::new_inner(endpoint.address(), endpoint.host().to_owned(), write_path)
    }

    /// Initialize cache using `read_path`, and write changes to `write_path`.
    pub async fn from_file(
        read_path: &Path,
        write_path: Option<Box<Path>>,
        hostname: String,
    ) -> Result<Self, Error> {
        log::debug!("Loading API addresses from {}", read_path.display());
        let address = read_address_file(read_path).await?;
        Ok(Self::new_inner(address, hostname, write_path))
    }

    fn new_inner(address: SocketAddr, hostname: String, write_path: Option<Box<Path>>) -> Self {
        let cache = AddressCacheInner::from_address(address);
        log::debug!("Using API address: {}", cache.address);

        Self {
            inner: Arc::new(Mutex::new(cache)),
            write_path: write_path.map(Arc::from),
            hostname,
        }
    }

    /// Returns the address if the hostname equals `API.host`. Otherwise, returns `None`.
    async fn resolve_hostname(&self, hostname: &str) -> Option<SocketAddr> {
        if hostname.eq_ignore_ascii_case(&self.hostname) {
            Some(self.get_address().await)
        } else {
            None
        }
    }

    /// Returns the currently selected address.
    pub async fn get_address(&self) -> SocketAddr {
        self.inner.lock().await.address
    }

    pub async fn set_address(&self, address: SocketAddr) -> Result<(), Error> {
        let mut inner = self.inner.lock().await;
        if address != inner.address {
            self.save_to_disk(&address).await?;
            inner.address = address;
        }
        Ok(())
    }

    async fn save_to_disk(&self, address: &SocketAddr) -> Result<(), Error> {
        let write_path = match self.write_path.as_ref() {
            Some(write_path) => write_path,
            None => return Ok(()),
        };

        let mut file = mullvad_fs::AtomicFile::new(&**write_path)
            .await
            .map_err(Error::Open)?;
        let mut contents = address.to_string();
        contents += "\n";
        file.write_all(contents.as_bytes())
            .await
            .map_err(Error::Write)?;
        file.finalize().await.map_err(Error::Write)
    }
}

#[derive(Clone, PartialEq, Eq)]
struct AddressCacheInner {
    address: SocketAddr,
}

impl AddressCacheInner {
    fn from_address(address: SocketAddr) -> Self {
        Self { address }
    }
}

async fn read_address_file(path: &Path) -> Result<SocketAddr, Error> {
    let mut file = fs::File::open(path).await.map_err(Error::Open)?;
    let mut address = String::new();
    file.read_to_string(&mut address)
        .await
        .map_err(Error::Read)?;
    address.trim().parse().map_err(|_| Error::Parse)
}
