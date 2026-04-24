use std::net::SocketAddr;

pub fn guard(addr: SocketAddr) -> Result<(), axum::http::StatusCode> {
    if addr.ip().is_loopback() {
        Ok(())
    } else {
        Err(axum::http::StatusCode::FORBIDDEN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(ip: [u8; 4]) -> SocketAddr {
        SocketAddr::from((ip, 1234))
    }

    #[test]
    fn loopback_ipv4_allowed() {
        assert!(guard(addr([127, 0, 0, 1])).is_ok());
    }

    #[test]
    fn loopback_ipv6_allowed() {
        let addr: SocketAddr = "[::1]:1234".parse().unwrap();
        assert!(guard(addr).is_ok());
    }

    #[test]
    fn lan_ip_rejected() {
        assert!(guard(addr([192, 168, 1, 5])).is_err());
    }

    #[test]
    fn public_ip_rejected() {
        assert!(guard(addr([8, 8, 8, 8])).is_err());
    }
}
