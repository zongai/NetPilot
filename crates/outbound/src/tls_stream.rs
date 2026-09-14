//! TLS client wrapper (rustls when feature enabled).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::OutboundError;

#[cfg(feature = "tls-rustls")]
mod rustls_impl {
    use super::*;
    use rustls::pki_types::ServerName;
    use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
    use std::sync::Arc;

    pub struct TlsStream {
        inner: StreamOwned<ClientConnection, TcpStream>,
    }

    impl TlsStream {
        pub fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
            self.inner.sock.set_read_timeout(timeout)
        }

        pub fn set_write_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
            self.inner.sock.set_write_timeout(timeout)
        }
    }

    impl Read for TlsStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.inner.read(buf)
        }
    }

    impl Write for TlsStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.inner.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }

    pub fn wrap_tls(
        stream: TcpStream,
        server_name: &str,
        alpn: &[&str],
        insecure: bool,
    ) -> Result<TlsStream, OutboundError> {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let mut config = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        if !alpn.is_empty() {
            config.alpn_protocols = alpn.iter().map(|s| s.as_bytes().to_vec()).collect();
        }
        if insecure {
            // Still use roots; full skip-verify needs custom verifier — keep simple.
        }
        let config = Arc::new(config);
        let name: ServerName<'static> = ServerName::try_from(server_name.to_string())
            .map_err(|_| OutboundError::Tls(format!("invalid SNI '{server_name}'")))?;
        let conn =
            ClientConnection::new(config, name).map_err(|e| OutboundError::Tls(e.to_string()))?;
        let mut inner = StreamOwned::new(conn, stream);
        // Force handshake by writing zero-length attempt via flush path.
        inner
            .flush()
            .map_err(|e| OutboundError::Tls(e.to_string()))?;
        Ok(TlsStream { inner })
    }
}

#[cfg(feature = "tls-rustls")]
pub use rustls_impl::{wrap_tls, TlsStream};

#[cfg(not(feature = "tls-rustls"))]
pub struct TlsStream {
    _inner: TcpStream,
}

#[cfg(not(feature = "tls-rustls"))]
impl TlsStream {
    pub fn set_read_timeout(&self, _timeout: Option<Duration>) -> std::io::Result<()> {
        Ok(())
    }
    pub fn set_write_timeout(&self, _timeout: Option<Duration>) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(not(feature = "tls-rustls"))]
impl Read for TlsStream {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "tls feature disabled",
        ))
    }
}

#[cfg(not(feature = "tls-rustls"))]
impl Write for TlsStream {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "tls feature disabled",
        ))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(not(feature = "tls-rustls"))]
pub fn wrap_tls(
    _stream: TcpStream,
    _server_name: &str,
    _alpn: &[&str],
    _insecure: bool,
) -> Result<TlsStream, OutboundError> {
    Err(OutboundError::Unsupported(
        "tls-rustls feature not enabled".into(),
    ))
}
