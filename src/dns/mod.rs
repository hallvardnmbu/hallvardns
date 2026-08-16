mod policy;
mod query;

use embassy_net::{
    udp::{PacketMetadata, UdpSocket},
    IpEndpoint, Ipv4Address, Stack,
};
use query::DnsQuery;

const UPSTREAM_DNS: Ipv4Address = Ipv4Address::new(1, 1, 1, 1);

pub async fn server(stack: Stack<'static>) {
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buffer = [0u8; 1232];
    let mut tx_buffer = [0u8; 1232];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);
    socket.bind(53).unwrap();

    let mut up_rx_meta = [PacketMetadata::EMPTY; 4];
    let mut up_tx_meta = [PacketMetadata::EMPTY; 4];
    let mut up_rx_buffer = [0u8; 1232];
    let mut up_tx_buffer = [0u8; 1232];
    let mut upstream = UdpSocket::new(stack, &mut up_rx_meta, &mut up_rx_buffer, &mut up_tx_meta, &mut up_tx_buffer);
    upstream.bind(0).unwrap();
    let upstream_endpoint = IpEndpoint::new(UPSTREAM_DNS.into(), 53);

    let mut buf = [0u8; 1232];

    loop {
        let Ok((len, remote)) = socket.recv_from(&mut buf).await else {
            continue;
        };

        let Some(query) = DnsQuery::parse(&buf[..len]) else {
            continue;
        };
        let name = query.name();

        if let Some(addr) = policy::local_mapping(name) {
            if let Some(n) = respond(&mut buf, query.question_end, Some(addr)) {
                let _ = socket.send_to(&buf[..n], remote.endpoint).await;
            }
            continue;
        }

        if policy::is_blocked(name) {
            if let Some(n) = respond(&mut buf, query.question_end, None) {
                let _ = socket.send_to(&buf[..n], remote.endpoint).await;
            }
            continue;
        }

        if upstream.send_to(&buf[..len], upstream_endpoint).await.is_err() {
            continue;
        }

        if let Ok((n, _)) = upstream.recv_from(&mut buf).await {
            let _ = socket.send_to(&buf[..n], remote.endpoint).await;
        }
    }
}

fn respond(buf: &mut [u8], question_end: usize, answer: Option<[u8; 4]>) -> Option<usize> {
    match answer {
        None => {
            buf[2] = 0x84;
            buf[3] = 0x03;
            buf[6..12].fill(0);
            Some(question_end)
        }
        Some(addr) => {
            let total_len = question_end.checked_add(16)?;
            if total_len > buf.len() { return None; }

            buf[2] = (buf[2] & 0x01) | 0x80 | 0x04;
            buf[3] &= 0x01;
            buf[6] = 0;
            buf[7] = 1;
            buf[8..12].fill(0);

            let mut pos = question_end;
            buf[pos] = 0xc0; buf[pos + 1] = 0x0c; pos += 2;
            buf[pos..pos + 2].copy_from_slice(&1u16.to_be_bytes()); pos += 2;
            buf[pos..pos + 2].copy_from_slice(&1u16.to_be_bytes()); pos += 2;
            buf[pos..pos + 4].copy_from_slice(&60u32.to_be_bytes()); pos += 4;
            buf[pos..pos + 2].copy_from_slice(&4u16.to_be_bytes()); pos += 2;
            buf[pos..pos + 4].copy_from_slice(&addr);

            Some(total_len)
        }
    }
}
