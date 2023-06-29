use std::net::SocketAddr;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

pub fn route_from_local(buf: &mut [u8]) ->(&mut[u8], SocketAddr) {
    (buf, "10.211.55.5:55001".parse::<SocketAddr>().unwrap())
}

pub fn route_from_remote(buf: &mut [u8]) ->(&mut[u8], Option<SocketAddr>) {
    (buf, None)
}