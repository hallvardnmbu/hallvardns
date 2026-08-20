use embassy_net::{
    udp::{PacketMetadata, UdpSocket},
    IpEndpoint, Ipv4Address, Stack,
};

const UPSTREAM: Ipv4Address = Ipv4Address::new(1, 1, 1, 1);

const LOCAL: &[(&str, [u8; 4])] = &[
    ("server.lan", [192, 168, 86, 89]),
];

// Local logging via UDP.
const LOGGER_IP: Ipv4Address = Ipv4Address::new(192, 168, 86, 130);
const LOGGER_PORT: u16 = 5514;

/// DNS server.
pub async fn serve(stack: Stack<'static>) {
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx = [0u8; 1232];
    let mut tx = [0u8; 1232];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx, &mut tx_meta, &mut tx);
    socket.bind(53).unwrap();

    let mut up_rx_meta = [PacketMetadata::EMPTY; 4];
    let mut up_tx_meta = [PacketMetadata::EMPTY; 4];
    let mut up_rx = [0u8; 1232];
    let mut up_tx = [0u8; 1232];
    let mut upstream = UdpSocket::new(stack, &mut up_rx_meta, &mut up_rx, &mut up_tx_meta, &mut up_tx);
    upstream.bind(0).unwrap();
    let upstream_endpoint = IpEndpoint::new(UPSTREAM.into(), 53);

    let mut log_rx_meta = [PacketMetadata::EMPTY; 1];
    let mut log_tx_meta = [PacketMetadata::EMPTY; 1];
    let mut log_rx = [0u8; 1];
    let mut log_tx = [0u8; 253]; // Max DNS name length.
    let mut logger = UdpSocket::new(stack, &mut log_rx_meta, &mut log_rx, &mut log_tx_meta, &mut log_tx);
    logger.bind(0).unwrap();
    let logger_endpoint = IpEndpoint::new(LOGGER_IP.into(), LOGGER_PORT);

    let mut buf = [0u8; 1232];

    esp_println::println!("listening...");
    loop {
        let Ok((len, remote)) = socket.recv_from(&mut buf).await else {
            continue;
        };

        let Some(query) = parse(&buf[..len]) else {
            continue;
        };

        let name = query.name();

        esp_println::println!("can you dig {:?} ?", name);

        // Log the queried name via UDP.
        let _ = logger.send_to(name.as_bytes(), logger_endpoint).await;


        if query.qtype != 1 {
            continue; // not an A query — nothing more to do
        }

        if let Some(addr) = local(name) {
            if let Some(len) = answer(&mut buf, query.end, Some(addr)) {
                esp_println::println!(" of course, issa local");
                let _ = socket.send_to(&buf[..len], remote.endpoint).await;
            }
            continue;
        }

        if upstream.send_to(&buf[..len], upstream_endpoint).await.is_ok() {
            if let Ok((len, _)) = upstream.recv_from(&mut buf).await {
                let _ = socket.send_to(&buf[..len], remote.endpoint).await;
            }
        }
    }
}

struct Query {
    name: [u8; 253],
    len: usize,
    end: usize,
    qtype: u16,
}

impl Query {
    fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.len]).unwrap_or("")
    }
}

fn parse(packet: &[u8]) -> Option<Query> {
    // Header must be at least 12 bytes + 5 bytes min for QNAME/QTYPE/QCLASS
    if packet.len() < 17 {
        return None;
    }

    // Must be a query (QR bit = 0)
    if packet[2] & 0x80 != 0 {
        return None;
    }

    // QDCOUNT must be 1 (packet[4..6] == [0, 1])
    if packet[4..6] != [0, 1] {
        return None;
    }

    let mut name = [0u8; 253];
    let mut len = 0;
    let mut pos = 12;

    loop {
        let label_len = *packet.get(pos)? as usize;
        pos += 1;

        if label_len == 0 {
            break;
        }

        // Handle pointers (0xc0) or overly long labels
        if label_len > 63 || (label_len & 0xc0) != 0 {
            return None;
        }

        if len != 0 {
            *name.get_mut(len)? = b'.';
            len += 1;
        }

        for &c in packet.get(pos..pos + label_len)? {
            *name.get_mut(len)? = c.to_ascii_lowercase();
            len += 1;
        }

        pos += label_len;
    }

    let qtype = u16::from_be_bytes(packet.get(pos..pos + 2)?.try_into().ok()?);
    let qclass = u16::from_be_bytes(packet.get(pos + 2..pos + 4)?.try_into().ok()?);
    if qclass != 1 {
        return None;
    }

    Some(Query {
        name,
        len,
        end: pos + 4,
        qtype,
    })
}

fn local(name: &str) -> Option<[u8; 4]> {
    LOCAL.iter()
        .find(|(domain, _)| *domain == name)
        .map(|(_, addr)| *addr)
}

fn answer(
    buf: &mut [u8],
    question_end: usize,
    addr: Option<[u8; 4]>,
) -> Option<usize> {
    match addr {
        None => {
            buf[2] = 0x84;
            buf[3] = 0x03;
            buf[6..12].fill(0);

            Some(question_end)
        }

        Some(addr) => {
            let end = question_end.checked_add(16)?;

            if end > buf.len() {
                return None;
            }

            buf[2] = (buf[2] & 1) | 0x80 | 0x04;
            buf[3] &= 1;

            buf[6] = 0;
            buf[7] = 1;
            buf[8..12].fill(0);

            let mut pos = question_end;

            buf[pos..pos + 2].copy_from_slice(&[0xc0, 0x0c]);
            pos += 2;

            buf[pos..pos + 2].copy_from_slice(&[0, 1]);
            pos += 2;

            buf[pos..pos + 2].copy_from_slice(&[0, 1]);
            pos += 2;

            buf[pos..pos + 4].copy_from_slice(&60u32.to_be_bytes());
            pos += 4;

            buf[pos..pos + 2].copy_from_slice(&[0, 4]);
            pos += 2;

            buf[pos..pos + 4].copy_from_slice(&addr);

            Some(end)
        }
    }
}
