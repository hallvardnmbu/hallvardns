use embassy_net::{
    udp::{PacketMetadata, UdpSocket},
    IpEndpoint, Ipv4Address, Stack,
};

const UPSTREAM: Ipv4Address = Ipv4Address::new(1, 1, 1, 1);

const BLOCKED: &[&str] = &[
    "blocked.com",
];

const LOCAL: &[(&str, [u8; 4])] = &[
    ("server.lan", [192, 168, 86, 89]),
];

pub async fn run(stack: Stack<'static>) {
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx = [0u8; 1232];
    let mut tx = [0u8; 1232];

    let mut socket =
        UdpSocket::new(stack, &mut rx_meta, &mut rx, &mut tx_meta, &mut tx);

    socket.bind(53).unwrap();

    let mut up_rx_meta = [PacketMetadata::EMPTY; 4];
    let mut up_tx_meta = [PacketMetadata::EMPTY; 4];
    let mut up_rx = [0u8; 1232];
    let mut up_tx = [0u8; 1232];

    let mut upstream =
        UdpSocket::new(stack, &mut up_rx_meta, &mut up_rx, &mut up_tx_meta, &mut up_tx);

    upstream.bind(0).unwrap();

    let upstream_endpoint = IpEndpoint::new(UPSTREAM.into(), 53);

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

        if let Some(addr) = local(name) {
            if let Some(len) = answer(&mut buf, query.end, Some(addr)) {
                esp_println::println!(" of course, issa local");
                let _ = socket.send_to(&buf[..len], remote.endpoint).await;
            }
            continue;
        }

        if blocked(name) {
            if let Some(len) = answer(&mut buf, query.end, None) {
                 esp_println::println!(" NO!");
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

    // Must be QTYPE = A (1) and QCLASS = IN (1)
    if packet.get(pos..pos + 4)? != [0, 1, 0, 1] {
        return None;
    }

    Some(Query {
        name,
        len,
        end: pos + 4,
    })
}

fn local(name: &str) -> Option<[u8; 4]> {
    LOCAL.iter()
        .find(|(domain, _)| *domain == name)
        .map(|(_, addr)| *addr)
}

fn blocked(name: &str) -> bool {
    if BLOCKED.binary_search(&name).is_ok() {
        return true;
    }

    let mut name = name;

    while let Some((_, rest)) = name.split_once('.') {
        if BLOCKED.binary_search(&rest).is_ok() {
            return true;
        }

        name = rest;
    }

    false
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
