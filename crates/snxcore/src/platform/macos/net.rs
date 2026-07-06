use std::{
    ffi::CStr,
    mem,
    net::{Ipv4Addr, Ipv6Addr},
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
    ptr,
    sync::atomic::{AtomicI32, Ordering},
};

use anyhow::anyhow;
use ipnet::Ipv4Net;
use libc::{c_char, c_int, c_ulong, c_void};
use nix::sys::{
    socket::{AddressFamily, SockFlag, SockType, setsockopt, socket, sockopt::ReceiveTimeout},
    time::TimeVal,
};
use tracing::debug;

use crate::platform::{DeviceConfig, NetworkInterface, StatsPoller};

// BSD `_IOW('i', n, T)` request encoding; sizeof(T) is baked into the request number, so the
// struct we hand to `ioctl` must be exactly that size (see the InAliasReq / libc::ifreq notes).
const fn iow(group: u8, num: u8, len: usize) -> c_ulong {
    const IOC_IN: c_ulong = 0x8000_0000;
    const IOCPARM_MASK: c_ulong = 0x1fff;
    IOC_IN | (((len as c_ulong) & IOCPARM_MASK) << 16) | ((group as c_ulong) << 8) | num as c_ulong
}

const SIOCSIFMTU: c_ulong = iow(b'i', 52, mem::size_of::<libc::ifreq>());
const SIOCAIFADDR: c_ulong = iow(b'i', 26, mem::size_of::<InAliasReq>());
const SIOCDIFADDR: c_ulong = iow(b'i', 25, mem::size_of::<libc::ifreq>());

// netinet/in_var.h: same 64-byte layout as `struct ifaliasreq` but with sockaddr_in members.
#[repr(C)]
struct InAliasReq {
    ifra_name: [c_char; libc::IFNAMSIZ],
    ifra_addr: libc::sockaddr_in,
    ifra_dstaddr: libc::sockaddr_in,
    ifra_mask: libc::sockaddr_in,
}

static ROUTE_SEQ: AtomicI32 = AtomicI32::new(1);

pub(super) fn next_seq() -> c_int {
    ROUTE_SEQ.fetch_add(1, Ordering::Relaxed)
}

pub(super) fn struct_bytes<T>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((v as *const T).cast::<u8>(), mem::size_of::<T>()) }
}

pub(super) fn sockaddr_in(addr: Ipv4Addr) -> libc::sockaddr_in {
    libc::sockaddr_in {
        sin_len: mem::size_of::<libc::sockaddr_in>() as u8,
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr {
            s_addr: u32::from_ne_bytes(addr.octets()),
        },
        sin_zero: [0; 8],
    }
}

pub(super) fn sockaddr_in6(addr: Ipv6Addr) -> libc::sockaddr_in6 {
    libc::sockaddr_in6 {
        sin6_len: mem::size_of::<libc::sockaddr_in6>() as u8,
        sin6_family: libc::AF_INET6 as libc::sa_family_t,
        sin6_port: 0,
        sin6_flowinfo: 0,
        sin6_addr: libc::in6_addr { s6_addr: addr.octets() },
        sin6_scope_id: 0,
    }
}

pub(super) fn sockaddr_dl(ifindex: u16) -> libc::sockaddr_dl {
    let mut sdl: libc::sockaddr_dl = unsafe { mem::zeroed() };
    sdl.sdl_len = mem::size_of::<libc::sockaddr_dl>() as u8;
    sdl.sdl_family = libc::AF_LINK as u8;
    sdl.sdl_index = ifindex;
    sdl
}

pub(super) fn route_socket() -> anyhow::Result<OwnedFd> {
    let fd = socket(AddressFamily::Route, SockType::Raw, SockFlag::empty(), None)?;
    // route_get() reads synchronously from async keepalive/scv loops; bound the wait so a lost
    // or never-matching reply cannot hang a Tokio worker indefinitely.
    setsockopt(&fd, ReceiveTimeout, &TimeVal::new(2, 0))?;
    Ok(fd)
}

fn inet_dgram_socket() -> anyhow::Result<OwnedFd> {
    Ok(socket(
        AddressFamily::Inet,
        SockType::Datagram,
        SockFlag::empty(),
        None,
    )?)
}

// PF_ROUTE sockaddrs are padded to a 4-byte boundary; a zero-length sockaddr still advances one word.
fn roundup(len: usize) -> usize {
    let word = mem::size_of::<u32>();
    if len == 0 { word } else { (len + word - 1) & !(word - 1) }
}

// `parts` must be supplied in ascending RTA_* bit order.
pub(super) fn build_route_message(rtm_type: u8, rtm_flags: c_int, rtm_seq: c_int, parts: &[(c_int, &[u8])]) -> Vec<u8> {
    let mut payload = Vec::new();
    let mut rtm_addrs = 0;
    for (bit, sa) in parts {
        rtm_addrs |= *bit;
        let start = payload.len();
        payload.extend_from_slice(sa);
        payload.resize(start + roundup(sa.len()), 0);
    }

    let total = mem::size_of::<libc::rt_msghdr>() + payload.len();
    let mut hdr: libc::rt_msghdr = unsafe { mem::zeroed() };
    hdr.rtm_msglen = total as u16;
    hdr.rtm_version = libc::RTM_VERSION as u8;
    hdr.rtm_type = rtm_type;
    hdr.rtm_flags = rtm_flags;
    hdr.rtm_addrs = rtm_addrs;
    hdr.rtm_seq = rtm_seq;

    let mut buf = Vec::with_capacity(total);
    buf.extend_from_slice(struct_bytes(&hdr));
    buf.extend_from_slice(&payload);
    buf
}

pub(super) fn send_route_message(fd: BorrowedFd<'_>, buf: &[u8]) -> std::io::Result<()> {
    nix::unistd::write(fd, buf).map_err(std::io::Error::from)?;
    Ok(())
}

pub(super) struct RouteReply {
    pub ifindex: u16,
    pub gateway: Option<Vec<u8>>,
}

pub(super) fn route_get(fd: BorrowedFd<'_>, dst: Ipv4Addr) -> anyhow::Result<RouteReply> {
    let seq = next_seq();
    let dst_sa = sockaddr_in(dst);
    let msg = build_route_message(libc::RTM_GET as u8, 0, seq, &[(libc::RTA_DST, struct_bytes(&dst_sa))]);
    send_route_message(fd, &msg)?;

    let pid = unsafe { libc::getpid() };
    let mut buf = [0u8; 2048];
    loop {
        // A receive timeout (set in route_socket) surfaces here as EAGAIN and is propagated.
        let n = nix::unistd::read(fd, &mut buf).map_err(std::io::Error::from)?;
        if n < mem::size_of::<libc::rt_msghdr>() {
            continue;
        }
        let hdr: libc::rt_msghdr = unsafe { std::ptr::read_unaligned(buf.as_ptr().cast()) };
        if hdr.rtm_type as c_int != libc::RTM_GET || hdr.rtm_seq != seq || hdr.rtm_pid != pid {
            continue;
        }
        if hdr.rtm_errno != 0 {
            return Err(std::io::Error::from_raw_os_error(hdr.rtm_errno).into());
        }

        let mut offset = mem::size_of::<libc::rt_msghdr>();
        let mut gateway = None;
        for i in 0..libc::RTAX_MAX {
            if hdr.rtm_addrs & (1 << i) == 0 || offset >= n {
                continue;
            }
            let sa_len = buf[offset] as usize;
            if i == libc::RTAX_GATEWAY && sa_len != 0 {
                gateway = Some(buf[offset..(offset + sa_len).min(n)].to_vec());
            }
            offset += roundup(sa_len);
        }

        return Ok(RouteReply {
            ifindex: hdr.rtm_index,
            gateway,
        });
    }
}

fn copy_ifname(dst: &mut [c_char; libc::IFNAMSIZ], name: &str) {
    let bytes = name.as_bytes();
    let n = bytes.len().min(dst.len() - 1);
    for (slot, byte) in dst.iter_mut().zip(&bytes[..n]) {
        *slot = *byte as c_char;
    }
}

fn ioctl<T>(fd: BorrowedFd<'_>, request: c_ulong, arg: &T) -> std::io::Result<()> {
    let rc = unsafe { libc::ioctl(fd.as_raw_fd(), request, arg as *const T) };
    if rc < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn set_mtu(fd: BorrowedFd<'_>, name: &str, mtu: u16) -> std::io::Result<()> {
    let mut req: libc::ifreq = unsafe { mem::zeroed() };
    copy_ifname(&mut req.ifr_name, name);
    req.ifr_ifru.ifru_mtu = mtu as c_int;
    ioctl(fd, SIOCSIFMTU, &req)
}

fn add_alias(fd: BorrowedFd<'_>, name: &str, net: Ipv4Net) -> std::io::Result<()> {
    let mut req: InAliasReq = unsafe { mem::zeroed() };
    copy_ifname(&mut req.ifra_name, name);
    req.ifra_addr = sockaddr_in(net.addr());
    // utun is point-to-point; set the peer address to the interface address itself.
    req.ifra_dstaddr = sockaddr_in(net.addr());
    req.ifra_mask = sockaddr_in(net.netmask());
    ioctl(fd, SIOCAIFADDR, &req)
}

fn del_address(fd: BorrowedFd<'_>, name: &str, addr: Ipv4Addr) -> std::io::Result<()> {
    let mut req: libc::ifreq = unsafe { mem::zeroed() };
    copy_ifname(&mut req.ifr_name, name);
    let sin = sockaddr_in(addr);
    req.ifr_ifru.ifru_addr = unsafe { mem::transmute_copy(&sin) };
    match ioctl(fd, SIOCDIFADDR, &req) {
        Err(e) if e.raw_os_error() == Some(libc::EADDRNOTAVAIL) => Ok(()),
        other => other,
    }
}

fn if_indextoname(index: u16) -> Option<String> {
    let mut buf = [0 as c_char; libc::IF_NAMESIZE];
    let ret = unsafe { libc::if_indextoname(index as libc::c_uint, buf.as_mut_ptr()) };
    if ret.is_null() {
        return None;
    }
    Some(unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned())
}

fn is_point_to_point(name: &str) -> bool {
    nix::ifaddrs::getifaddrs().is_ok_and(|addrs| {
        addrs
            .filter(|ifa| ifa.interface_name == name)
            .any(|ifa| ifa.flags.contains(nix::net::if_::InterfaceFlags::IFF_POINTOPOINT))
    })
}

// sysctl(CTL_NET, PF_ROUTE, 0, AF_INET, NET_RT_DUMP, 0) returns the whole IPv4 routing table as a
// sequence of rt_msghdr-prefixed messages, each padded to rtm_msglen.
fn dump_inet_routes() -> anyhow::Result<Vec<u8>> {
    let mut mib = [libc::CTL_NET, libc::PF_ROUTE, 0, libc::AF_INET, libc::NET_RT_DUMP, 0];
    let mut needed: libc::size_t = 0;
    let rc = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as libc::c_uint,
            ptr::null_mut(),
            &mut needed,
            ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    let mut buf = vec![0u8; needed];
    let rc = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as libc::c_uint,
            buf.as_mut_ptr().cast(),
            &mut needed,
            ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    buf.truncate(needed);
    Ok(buf)
}

// Number of set bits in a routing-socket netmask sockaddr. These sockaddrs are truncated to their
// significant bytes (sa_len), so a missing tail counts as zero and a /0 default reports sa_len 0.
fn netmask_prefix_len(sa: &[u8]) -> u32 {
    // sockaddr_in holds sin_addr at bytes 4..8
    sa.get(4..8.min(sa.len()))
        .map_or(0, |b| b.iter().map(|x| x.count_ones()).sum())
}

fn route_is_default(body: &[u8], rtm_addrs: c_int) -> bool {
    let mut offset = 0;
    let mut have_dst = false;
    let mut dst_zero = false;
    let mut netmask_zero = true;
    for i in 0..libc::RTAX_MAX {
        if rtm_addrs & (1 << i) == 0 {
            continue;
        }
        if offset >= body.len() {
            break;
        }
        let sa_len = body[offset] as usize;
        let sa = &body[offset..(offset + sa_len).min(body.len())];
        if i == libc::RTAX_DST {
            have_dst = true;
            dst_zero = sa.get(4..8).is_none_or(|b| b.iter().all(|&x| x == 0));
        } else if i == libc::RTAX_NETMASK {
            netmask_zero = netmask_prefix_len(sa) == 0;
        }
        offset += roundup(sa_len);
    }
    have_dst && dst_zero && netmask_zero
}

// RTM_GET cannot be used here: it does longest-prefix match, so a covering half-default (e.g. a
// VPN's 0.0.0.0/1) would shadow the real default and bind us to a tunnel's source address.
// Enumerate the table and pick the /0 gateway route on a non-point-to-point interface instead.
fn default_route_interface() -> anyhow::Result<String> {
    let buf = dump_inet_routes()?;
    let hdr_len = mem::size_of::<libc::rt_msghdr>();
    let mut offset = 0;
    while offset + hdr_len <= buf.len() {
        let hdr: libc::rt_msghdr = unsafe { ptr::read_unaligned(buf[offset..].as_ptr().cast()) };
        let msglen = hdr.rtm_msglen as usize;
        if msglen < hdr_len || offset + msglen > buf.len() {
            break;
        }

        let is_default = hdr.rtm_flags & libc::RTF_UP != 0
            && hdr.rtm_flags & libc::RTF_GATEWAY != 0
            && route_is_default(&buf[offset + hdr_len..offset + msglen], hdr.rtm_addrs);

        if is_default
            && let Some(name) = if_indextoname(hdr.rtm_index)
            && !is_point_to_point(&name)
        {
            return Ok(name);
        }

        offset += msglen;
    }

    Err(anyhow!(i18n::tr!("error-cannot-determine-ip")))
}

// Previous value of net.inet.ip.forwarding captured before we enabled it, or -1 if untouched.
static SAVED_IP_FORWARDING: AtomicI32 = AtomicI32::new(-1);

const IP_FORWARDING_CTL: &CStr = c"net.inet.ip.forwarding";

fn get_ip_forwarding() -> std::io::Result<c_int> {
    let mut value: c_int = 0;
    let mut size = mem::size_of::<c_int>();
    let rc = unsafe {
        libc::sysctlbyname(
            IP_FORWARDING_CTL.as_ptr(),
            (&mut value as *mut c_int).cast::<c_void>(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(value)
}

fn set_ip_forwarding(value: c_int) -> std::io::Result<()> {
    let mut value = value;
    let rc = unsafe {
        libc::sysctlbyname(
            IP_FORWARDING_CTL.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            (&mut value as *mut c_int).cast::<c_void>(),
            mem::size_of::<c_int>(),
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

// Pre-snx forwarding value, persisted while enabled so a crash-restart (which loses the in-memory
// SAVED_IP_FORWARDING) can still restore it. In /var/run, reset on reboot like the sysctl.
const FORWARDING_MARKER: &str = "/var/run/snx-rs.forwarding";

// Restore net.inet.ip.forwarding from the marker left by a previous instance that enabled it and
// exited uncleanly, then remove the marker. A no-op on a clean start (no marker present).
pub(super) fn restore_forwarding_from_marker() {
    let Ok(contents) = std::fs::read_to_string(FORWARDING_MARKER) else {
        return;
    };
    if let Ok(value) = contents.trim().parse::<c_int>()
        && let Err(e) = set_ip_forwarding(value)
    {
        debug!("Failed to restore net.inet.ip.forwarding from marker: {e}");
    }
    let _ = std::fs::remove_file(FORWARDING_MARKER);
}

#[derive(Default)]
pub struct MacosNetworkInterface;

impl MacosNetworkInterface {
    pub fn new() -> Self {
        Self
    }
}

impl NetworkInterface for MacosNetworkInterface {
    async fn start_network_state_monitoring(&self) -> anyhow::Result<()> {
        // is_online() probes the routing table on demand, so no background watcher is needed.
        Ok(())
    }

    async fn get_default_ipv4(&self) -> anyhow::Result<Ipv4Addr> {
        let name = default_route_interface()?;

        for ifa in nix::ifaddrs::getifaddrs()? {
            if ifa.interface_name == name
                && let Some(ip) = ifa.address.and_then(|a| a.as_sockaddr_in().map(|s| s.ip()))
            {
                debug!("Default route via {name} with address {ip}");
                return Ok(ip);
            }
        }

        Err(anyhow!(i18n::tr!("error-cannot-determine-ip")))
    }

    async fn delete_device(&self, _device_name: &str) -> anyhow::Result<()> {
        // Restore the global IP forwarding value if configure_device changed it.
        let saved = SAVED_IP_FORWARDING.swap(-1, Ordering::SeqCst);
        if saved >= 0 {
            if let Err(e) = set_ip_forwarding(saved) {
                debug!("Failed to restore net.inet.ip.forwarding: {e}");
            }
            let _ = std::fs::remove_file(FORWARDING_MARKER);
        }
        // utun disappears when the tun fd is dropped
        Ok(())
    }

    async fn configure_device(&self, device_config: &DeviceConfig) -> anyhow::Result<()> {
        debug!("Configuring device: {:?}", device_config);
        let sock = inet_dgram_socket()?;
        set_mtu(sock.as_fd(), &device_config.name, device_config.mtu)?;
        add_alias(sock.as_fd(), &device_config.name, device_config.address)?;

        if device_config.allow_forwarding {
            // Global sysctl, not per-interface like Linux; save the previous value in memory (for
            // delete_device) and on disk (for a crash-restart) before enabling.
            let previous = get_ip_forwarding()?;
            SAVED_IP_FORWARDING.store(previous, Ordering::SeqCst);
            let _ = std::fs::write(FORWARDING_MARKER, previous.to_string());
            set_ip_forwarding(1)?;
        }

        Ok(())
    }

    async fn replace_ip_address(
        &self,
        device_name: &str,
        old_address: Ipv4Net,
        new_address: Ipv4Net,
    ) -> anyhow::Result<()> {
        let sock = inet_dgram_socket()?;
        add_alias(sock.as_fd(), device_name, new_address)?;
        del_address(sock.as_fd(), device_name, old_address.addr())?;
        Ok(())
    }

    async fn new_stats_poller(&self, device_name: &str) -> anyhow::Result<impl StatsPoller + Send + Sync + 'static> {
        super::stats::MacosStatsPoller::new(device_name)
    }

    fn is_online(&self) -> bool {
        // A resolvable route toward the default address is enough signal that we have connectivity.
        route_socket()
            .and_then(|sock| route_get(sock.as_fd(), Ipv4Addr::UNSPECIFIED))
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use crate::platform::NetworkInterface;

    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *seed
    }

    fn push_sa(body: &mut Vec<u8>, sa: &[u8]) {
        let start = body.len();
        body.extend_from_slice(sa);
        body.resize(start + super::roundup(sa.len()), 0);
    }

    #[tokio::test]
    async fn get_default_ipv4_returns_physical_address() {
        let ip = super::MacosNetworkInterface::new()
            .get_default_ipv4()
            .await
            .expect("default IPv4 lookup should succeed without root");
        println!("get_default_ipv4 = {ip}");
        assert!(!ip.is_unspecified(), "expected a real address, got {ip}");
        assert!(!ip.is_loopback(), "expected a non-loopback address, got {ip}");
    }

    #[test]
    fn roundup_pads_to_word() {
        assert_eq!(super::roundup(0), 4);
        for len in 1..=256usize {
            let r = super::roundup(len);
            assert!(r >= len && r.is_multiple_of(4) && r < len + 4, "roundup({len}) = {r}");
        }
    }

    #[test]
    fn netmask_prefix_len_known_and_truncated() {
        let mut sa = vec![8u8, libc::AF_INET as u8, 0, 0, 0xff, 0xff, 0xff, 0x00];
        assert_eq!(super::netmask_prefix_len(&sa), 24);
        sa[4..8].copy_from_slice(&[0xff; 4]);
        assert_eq!(super::netmask_prefix_len(&sa), 32);
        assert_eq!(super::netmask_prefix_len(&[]), 0);
        assert_eq!(super::netmask_prefix_len(&[0, 2, 0]), 0);
    }

    #[test]
    fn netmask_prefix_len_never_panics() {
        let mut seed = 0xdead_beef_0000_0001;
        for _ in 0..5000 {
            let len = (lcg(&mut seed) % 16) as usize;
            let buf: Vec<u8> = (0..len).map(|_| (lcg(&mut seed) >> 33) as u8).collect();
            assert!(super::netmask_prefix_len(&buf) <= 32);
        }
    }

    #[test]
    fn route_is_default_detects_zero_dst_and_mask() {
        let gw = super::sockaddr_in(Ipv4Addr::new(192, 168, 1, 1));
        let mask = super::sockaddr_in(Ipv4Addr::UNSPECIFIED);
        let addrs = (1 << libc::RTAX_DST) | (1 << libc::RTAX_GATEWAY) | (1 << libc::RTAX_NETMASK);

        let dst = super::sockaddr_in(Ipv4Addr::UNSPECIFIED);
        let mut body = Vec::new();
        push_sa(&mut body, super::struct_bytes(&dst));
        push_sa(&mut body, super::struct_bytes(&gw));
        push_sa(&mut body, super::struct_bytes(&mask));
        assert!(super::route_is_default(&body, addrs));

        let dst = super::sockaddr_in(Ipv4Addr::new(10, 0, 0, 0));
        let mut body = Vec::new();
        push_sa(&mut body, super::struct_bytes(&dst));
        push_sa(&mut body, super::struct_bytes(&gw));
        push_sa(&mut body, super::struct_bytes(&mask));
        assert!(!super::route_is_default(&body, addrs));
    }

    #[test]
    fn route_is_default_never_panics() {
        let mut seed = 0xfeed_face_0000_0003;
        for _ in 0..5000 {
            let len = (lcg(&mut seed) % 200) as usize;
            let body: Vec<u8> = (0..len).map(|_| (lcg(&mut seed) >> 33) as u8).collect();
            let addrs = (lcg(&mut seed) & 0xff) as libc::c_int;
            let _ = super::route_is_default(&body, addrs);
        }
    }

    #[test]
    fn build_route_message_is_well_formed() {
        let dst = super::sockaddr_in(Ipv4Addr::new(10, 1, 2, 0));
        let mask = super::sockaddr_in(Ipv4Addr::new(255, 255, 255, 0));
        let msg = super::build_route_message(
            libc::RTM_ADD as u8,
            libc::RTF_UP | libc::RTF_STATIC,
            42,
            &[
                (libc::RTA_DST, super::struct_bytes(&dst)),
                (libc::RTA_NETMASK, super::struct_bytes(&mask)),
            ],
        );
        let hdr: libc::rt_msghdr = unsafe { std::ptr::read_unaligned(msg.as_ptr().cast()) };
        assert_eq!(hdr.rtm_msglen as usize, msg.len());
        assert_eq!(hdr.rtm_type, libc::RTM_ADD as u8);
        assert_eq!(hdr.rtm_seq, 42);
        assert_eq!(hdr.rtm_addrs, libc::RTA_DST | libc::RTA_NETMASK);
        assert_eq!(hdr.rtm_version, libc::RTM_VERSION as u8);
    }

    #[test]
    fn sockaddr_in_layout() {
        let sa = super::sockaddr_in(Ipv4Addr::new(1, 2, 3, 4));
        assert_eq!(sa.sin_len, std::mem::size_of::<libc::sockaddr_in>() as u8);
        assert_eq!(sa.sin_family, libc::AF_INET as libc::sa_family_t);
        assert_eq!(sa.sin_addr.s_addr, u32::from_ne_bytes([1, 2, 3, 4]));
    }
}
