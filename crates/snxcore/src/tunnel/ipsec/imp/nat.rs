use std::net::Ipv4Addr;

use ipnet::Ipv4Net;

const ICMP: u8 = 1;
const TCP: u8 = 6;
const UDP: u8 = 17;

const SOURCE: usize = 12;
const DESTINATION: usize = 16;

/// One-to-one address translation for a gateway that assigns no Office Mode address. Like the Windows
/// client, the tunnel interface gets a local placeholder, while the gateway only ever sees the session address.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StaticNat {
    local: Ipv4Addr,
    session: Ipv4Addr,
}

impl StaticNat {
    pub(crate) fn new(local: Ipv4Addr, session: Ipv4Addr) -> Self {
        Self { local, session }
    }

    pub(crate) fn local(&self) -> Ipv4Addr {
        self.local
    }

    pub(crate) fn outbound(&self, packet: &mut [u8]) {
        translate(packet, SOURCE, self.local, self.session);
    }

    pub(crate) fn inbound(&self, packet: &mut [u8]) {
        translate(packet, DESTINATION, self.session, self.local);
    }
}

/// The second host of the first /30 in 192.168.0.0/16, as the Windows client picks it, skipping blocks
/// that overlap the given networks or the /24 around the session address.
pub(crate) fn placeholder_address(session: Ipv4Addr, excluded: &[Ipv4Net]) -> Ipv4Addr {
    let fallback = Ipv4Addr::new(192, 168, 0, 2);
    let pool = Ipv4Net::new(Ipv4Addr::new(192, 168, 0, 0), 16).unwrap_or_default();
    let lan = Ipv4Net::new(session, 24).unwrap_or_default().trunc();
    let overlaps = |a: &Ipv4Net, b: &Ipv4Net| a.contains(&b.network()) || b.contains(&a.network());

    pool.subnets(30)
        .ok()
        .and_then(|mut blocks| {
            blocks.find(|block| !overlaps(block, &lan) && !excluded.iter().any(|net| overlaps(block, net)))
        })
        .map_or(fallback, |block| Ipv4Addr::from(u32::from(block.network()) + 2))
}

fn header_len(packet: &[u8]) -> Option<usize> {
    let first = *packet.first()?;
    let len = usize::from(first & 0x0f) * 4;
    (first >> 4 == 4 && len >= 20 && packet.len() >= len).then_some(len)
}

fn is_first_fragment(packet: &[u8]) -> bool {
    packet[6] & 0x1f == 0 && packet[7] == 0
}

fn is_icmp_error(icmp_type: u8) -> bool {
    matches!(icmp_type, 3 | 4 | 5 | 11 | 12)
}

/// RFC 1624, eqn. 3.
fn update_checksum(buf: &mut [u8], at: usize, old: &[u8], new: &[u8]) {
    let mut sum = u32::from(!u16::from_be_bytes([buf[at], buf[at + 1]]));
    for (old, new) in old.as_chunks::<2>().0.iter().zip(new.as_chunks::<2>().0) {
        sum += u32::from(!u16::from_be_bytes(*old));
        sum += u32::from(u16::from_be_bytes(*new));
    }
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    buf[at..at + 2].copy_from_slice(&(!(sum as u16)).to_be_bytes());
}

fn translate(packet: &mut [u8], field: usize, from: Ipv4Addr, to: Ipv4Addr) {
    let Some(len) = header_len(packet) else {
        return;
    };
    if packet[field..field + 4] != from.octets() {
        return;
    }

    packet[field..field + 4].copy_from_slice(&to.octets());
    update_checksum(packet, 10, &from.octets(), &to.octets());

    if !is_first_fragment(packet) {
        return;
    }

    let protocol = packet[9];
    let payload = &mut packet[len..];

    match protocol {
        TCP if payload.len() >= 18 => update_checksum(payload, 16, &from.octets(), &to.octets()),
        // A zero UDP checksum means none was computed; a computed zero is sent as all ones.
        UDP if payload.len() >= 8 && payload[6..8] != [0, 0] => {
            update_checksum(payload, 6, &from.octets(), &to.octets());
            if payload[6..8] == [0, 0] {
                payload[6..8].copy_from_slice(&[0xff, 0xff]);
            }
        }
        // An ICMP error quotes a packet that travelled the other way, so its other address is ours.
        ICMP if payload.len() >= 8 && is_icmp_error(payload[0]) => {
            let quoted = if field == SOURCE { DESTINATION } else { SOURCE };
            translate_quoted(payload, quoted, from, to);
        }
        _ => {}
    }
}

fn translate_quoted(icmp: &mut [u8], field: usize, from: Ipv4Addr, to: Ipv4Addr) {
    if header_len(&icmp[8..]).is_none() || icmp[8 + field..8 + field + 4] != from.octets() {
        return;
    }

    let old_checksum = [icmp[18], icmp[19]];
    icmp[8 + field..8 + field + 4].copy_from_slice(&to.octets());
    update_checksum(icmp, 18, &from.octets(), &to.octets());
    let new_checksum = [icmp[18], icmp[19]];

    update_checksum(icmp, 2, &from.octets(), &to.octets());
    update_checksum(icmp, 2, &old_checksum, &new_checksum);
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCAL: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 2);
    const SESSION: Ipv4Addr = Ipv4Addr::new(198, 51, 100, 7);
    const PEER: Ipv4Addr = Ipv4Addr::new(203, 0, 113, 9);

    fn sum(data: &[u8]) -> u32 {
        data.chunks(2)
            .map(|c| u32::from(u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)])))
            .sum()
    }

    fn fold(mut sum: u32) -> u16 {
        while sum > 0xffff {
            sum = (sum & 0xffff) + (sum >> 16);
        }
        !(sum as u16)
    }

    fn ipv4(src: Ipv4Addr, dst: Ipv4Addr, protocol: u8, payload: &[u8]) -> Vec<u8> {
        let mut packet = vec![0x45, 0, 0, 0, 0x12, 0x34, 0x40, 0, 64, protocol, 0, 0];
        packet[2..4].copy_from_slice(&((20 + payload.len()) as u16).to_be_bytes());
        packet.extend(src.octets());
        packet.extend(dst.octets());
        let checksum = fold(sum(&packet));
        packet[10..12].copy_from_slice(&checksum.to_be_bytes());
        packet.extend(payload);
        packet
    }

    fn transport_checksum(packet: &[u8]) -> u16 {
        let payload = &packet[20..];
        let pseudo = sum(&packet[12..20]) + u32::from(packet[9]) + payload.len() as u32;
        fold(pseudo + sum(payload))
    }

    fn with_transport_checksum(mut packet: Vec<u8>, at: usize) -> Vec<u8> {
        let checksum = transport_checksum(&packet);
        packet[20 + at..20 + at + 2].copy_from_slice(&checksum.to_be_bytes());
        packet
    }

    fn tcp(src: Ipv4Addr, dst: Ipv4Addr) -> Vec<u8> {
        let mut segment = vec![0u8; 20];
        segment[0..4].copy_from_slice(&[0xc3, 0x50, 0, 22]);
        segment[12] = 0x50;
        segment[13] = 0x02;
        with_transport_checksum(ipv4(src, dst, TCP, &segment), 16)
    }

    fn udp(src: Ipv4Addr, dst: Ipv4Addr, data: &[u8]) -> Vec<u8> {
        let mut datagram = vec![0x9c, 0x40, 0x47, 0x3a, 0, 0, 0, 0];
        datagram[4..6].copy_from_slice(&((8 + data.len()) as u16).to_be_bytes());
        datagram.extend(data);
        with_transport_checksum(ipv4(src, dst, UDP, &datagram), 6)
    }

    fn assert_valid(packet: &[u8]) {
        assert_eq!(fold(sum(&packet[..20])), 0, "IPv4 header checksum");
        match packet[9] {
            TCP | UDP => assert_eq!(transport_checksum(packet), 0, "transport checksum"),
            ICMP => assert_eq!(fold(sum(&packet[20..])), 0, "ICMP checksum"),
            _ => {}
        }
    }

    #[test]
    fn outbound_tcp_gets_the_session_source() {
        let mut packet = tcp(LOCAL, PEER);

        StaticNat::new(LOCAL, SESSION).outbound(&mut packet);

        assert_eq!(packet[12..16], SESSION.octets());
        assert_eq!(packet, tcp(SESSION, PEER));
        assert_valid(&packet);
    }

    #[test]
    fn inbound_udp_gets_the_local_destination() {
        let mut packet = udp(PEER, SESSION, b"keepalive reply");

        StaticNat::new(LOCAL, SESSION).inbound(&mut packet);

        assert_eq!(packet, udp(PEER, LOCAL, b"keepalive reply"));
        assert_valid(&packet);
    }

    #[test]
    fn udp_without_checksum_keeps_none() {
        let mut packet = udp(PEER, SESSION, b"data");
        packet[26..28].copy_from_slice(&[0, 0]);

        StaticNat::new(LOCAL, SESSION).inbound(&mut packet);

        assert_eq!(packet[16..20], LOCAL.octets());
        assert_eq!(packet[26..28], [0, 0]);
    }

    #[test]
    fn other_addresses_are_left_alone() {
        let nat = StaticNat::new(LOCAL, SESSION);

        let mut outbound = tcp(PEER, LOCAL);
        nat.outbound(&mut outbound);
        assert_eq!(outbound, tcp(PEER, LOCAL));

        let mut inbound = tcp(SESSION, PEER);
        nat.inbound(&mut inbound);
        assert_eq!(inbound, tcp(SESSION, PEER));

        let mut ipv6 = vec![0x60; 48];
        nat.outbound(&mut ipv6);
        assert_eq!(ipv6, vec![0x60; 48]);
    }

    #[test]
    fn later_fragments_only_change_the_ip_header() {
        let mut packet = tcp(LOCAL, PEER);
        packet[6..8].copy_from_slice(&[0x00, 0xb9]);
        let checksum = fold(sum(&[&packet[..10], &[0, 0], &packet[12..20]].concat()));
        packet[10..12].copy_from_slice(&checksum.to_be_bytes());
        let transport = packet[20..].to_vec();

        StaticNat::new(LOCAL, SESSION).outbound(&mut packet);

        assert_eq!(packet[12..16], SESSION.octets());
        assert_eq!(fold(sum(&packet[..20])), 0);
        assert_eq!(packet[20..], transport);
    }

    #[test]
    fn inbound_icmp_error_quotes_the_local_address() {
        let quoted = udp(SESSION, PEER, b"probe");
        let mut icmp = vec![3, 4, 0, 0, 0, 0, 0x05, 0x14];
        icmp.extend(&quoted[..28]);
        let checksum = fold(sum(&icmp));
        icmp[2..4].copy_from_slice(&checksum.to_be_bytes());
        let mut packet = ipv4(PEER, SESSION, ICMP, &icmp);

        StaticNat::new(LOCAL, SESSION).inbound(&mut packet);

        assert_eq!(packet[16..20], LOCAL.octets());
        assert_eq!(packet[20 + 8 + 12..20 + 8 + 16], LOCAL.octets());
        assert_eq!(fold(sum(&packet[28..48])), 0, "quoted IPv4 header checksum");
        assert_valid(&packet);
    }

    #[test]
    fn outbound_icmp_error_quotes_the_session_address() {
        let quoted = udp(PEER, LOCAL, b"probe");
        let mut icmp = vec![3, 3, 0, 0, 0, 0, 0, 0];
        icmp.extend(&quoted[..28]);
        let checksum = fold(sum(&icmp));
        icmp[2..4].copy_from_slice(&checksum.to_be_bytes());
        let mut packet = ipv4(LOCAL, PEER, ICMP, &icmp);

        StaticNat::new(LOCAL, SESSION).outbound(&mut packet);

        assert_eq!(packet[12..16], SESSION.octets());
        assert_eq!(packet[20 + 8 + 16..20 + 8 + 20], SESSION.octets());
        assert_eq!(fold(sum(&packet[28..48])), 0, "quoted IPv4 header checksum");
        assert_valid(&packet);
    }

    #[test]
    fn placeholder_follows_the_windows_client() {
        let encryption_domain = ["10.0.0.0/8".parse().unwrap(), "172.16.0.0/12".parse().unwrap()];

        assert_eq!(placeholder_address(SESSION, &encryption_domain), LOCAL);
    }

    #[test]
    fn placeholder_avoids_the_encryption_domain_and_the_local_network() {
        let encryption_domain = ["192.168.0.0/30".parse().unwrap()];
        assert_eq!(
            placeholder_address(SESSION, &encryption_domain),
            Ipv4Addr::new(192, 168, 0, 6)
        );

        assert_eq!(
            placeholder_address(Ipv4Addr::new(192, 168, 0, 23), &[]),
            Ipv4Addr::new(192, 168, 1, 2)
        );
    }
}
