pub struct DnsQuery {
    name: [u8; 253],
    name_len: usize,
    pub question_end: usize,
}

impl DnsQuery {
    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }

    pub fn parse(packet: &[u8]) -> Option<Self> {
        if packet.len() < 12 || packet[2] & 0x80 != 0 || packet[4] != 0 || packet[5] != 1 {
            return None;
        }

        let mut pos = 12;
        let mut name = [0u8; 253];
        let mut name_len = 0;

        loop {
            let label_len = *packet.get(pos)? as usize;
            pos += 1;
            if label_len == 0 { break; }
            if label_len & 0xc0 != 0 || label_len > 63 || pos + label_len > packet.len() {
                return None;
            }

            if name_len != 0 {
                *name.get_mut(name_len)? = b'.';
                name_len += 1;
            }
            if name_len + label_len > name.len() { return None; }
            for &c in &packet[pos..pos + label_len] {
                name[name_len] = c.to_ascii_lowercase();
                name_len += 1;
            }
            pos += label_len;
        }

        if pos + 4 > packet.len() { return None; }
        Some(Self { name, name_len, question_end: pos + 4 })
    }
}
