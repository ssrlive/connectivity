use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub fn is_port_reachable_with_timeout<A: ToSocketAddrs>(address: A, timeout: Duration) -> bool {
    match address.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(address) = addrs.next() {
                if TcpStream::connect_timeout(&address, timeout).is_ok() {
                    return true;
                }
            }
            false
        }
        Err(_err) => false,
    }
}
