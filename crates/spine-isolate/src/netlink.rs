//! P4(a)'s source — and RF §7.1 makes it **normative rather than an
//! implementation note**.
//!
//! Verbatim: *"§12 leaves the syscall that **creates** M1's namespaces to the
//! implementation, and this does not withdraw that; what a **test** reads is a
//! different question, because a test that can be aimed at the wrong namespace
//! is not a test. `unshare(2)` does not re-derive an already-mounted `sysfs`,
//! so `/sys/class/net` inside a correctly isolated namespace continues to list
//! the **job's own** interfaces."*
//!
//! The measurement that settled it, RF §7.1, from inside a namespace whose
//! netlink view is ten devices:
//!
//! ```text
//! /proc/net/dev  : lo tunl0 gre0 gretap0 erspan0 ip_vti0 ip6_vti0 sit0 ip6tnl0 ip6gre0
//! /sys/class/net : bonding_masters erspan0 eth0 gre0 gretap0 ip6gre0 ip6tnl0 ip…
//! ```
//!
//! `eth0` is the job's external interface and it is **not in the namespace**. A
//! probe reading `sysfs` reports a false **fail** here and, on a runtime that
//! did re-derive it, could report a false **pass** for a namespace it never
//! entered.
//!
//! Everything below the socket is a pure function of bytes, which is what lets
//! the published measurement be reproduced as a test on a host with no netlink
//! at all.

use core::fmt;

/// `<linux/netlink.h>`, `<linux/rtnetlink.h>`, `<net/if.h>`. Kernel ABI, not
/// this implementation's choice — RF §7.1 names `RTM_GETLINK` and
/// `RTM_GETADDR` and nothing else about the wire.
pub const NLMSG_NOOP: u16 = 1;
pub const NLMSG_ERROR: u16 = 2;
pub const NLMSG_DONE: u16 = 3;
pub const RTM_NEWLINK: u16 = 16;
pub const RTM_GETLINK: u16 = 18;
pub const RTM_NEWADDR: u16 = 20;
pub const RTM_GETADDR: u16 = 22;

const NLM_F_REQUEST: u16 = 0x001;
const NLM_F_ACK: u16 = 0x004;
/// `NLM_F_ROOT | NLM_F_MATCH`.
const NLM_F_DUMP: u16 = 0x300;

const IFLA_IFNAME: u16 = 3;
const IFA_ADDRESS: u16 = 1;
const IFA_LOCAL: u16 = 2;

pub const IFF_UP: u32 = 0x1;
pub const IFF_LOOPBACK: u32 = 0x8;

/// Linux's `AF_INET`/`AF_INET6`. **`AF_INET6` is 10 on Linux and 30 on Darwin**,
/// and this crate is compiled on both: the constant is written out here rather
/// than taken from the host's headers, because the bytes being parsed are
/// always Linux's whatever machine parses them.
pub const AF_INET: u8 = 2;
pub const AF_INET6: u8 = 10;
pub const AF_UNSPEC: u8 = 0;

const NLMSGHDR_LEN: usize = 16;
const IFINFOMSG_LEN: usize = 16;
const IFADDRMSG_LEN: usize = 8;
const RTATTR_LEN: usize = 4;

/// `NLMSG_ALIGN` / `RTA_ALIGN`, both 4.
const fn align4(n: usize) -> usize {
    n.div_ceil(4) * 4
}

/// One interface, as netlink reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub index: u32,
    pub name: String,
    /// `ifi_flags`. P4(a) reads **`IFF_UP`** out of this and nothing else:
    /// RF §13 R36 replaced the device count with the reachable set, *"because a
    /// veth pair, a bridge or an inherited interface must be up and addressed to
    /// carry traffic, and a down, address-less `gre0` cannot carry any"*.
    pub flags: u32,
}

impl Link {
    pub fn is_up(&self) -> bool {
        self.flags & IFF_UP != 0
    }

    pub fn is_loopback(&self) -> bool {
        self.flags & IFF_LOOPBACK != 0
    }
}

/// One address of any family, as netlink reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addr {
    pub index: u32,
    pub family: u8,
    pub prefix_len: u8,
    pub bytes: Vec<u8>,
}

impl Addr {
    /// `127.0.0.1/8` — the only IPv4 address P4(a) admits.
    pub fn is_the_loopback_v4(&self) -> bool {
        self.family == AF_INET && self.prefix_len == 8 && self.bytes == [127, 0, 0, 1]
    }

    /// `::1/128` — the only IPv6 address P4(a) admits.
    pub fn is_the_loopback_v6(&self) -> bool {
        let mut want = [0u8; 16];
        want[15] = 1;
        self.family == AF_INET6 && self.prefix_len == 128 && self.bytes == want
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetlinkError {
    /// A message header that runs past the end of the buffer, or claims a
    /// length below the header itself. Refused rather than skipped: a dump this
    /// parser cannot fully account for is a dump P4(a) must not decide on.
    Truncated,
    /// `NLMSG_ERROR`. The kernel's negated errno.
    Kernel(i32),
    /// An `IFLA_IFNAME` that is not UTF-8. Interface names are kernel-side
    /// ASCII, so this is a corrupt dump rather than a naming convention.
    NameNotUtf8,
}

impl fmt::Display for NetlinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetlinkError::Truncated => f.write_str("netlink dump truncated"),
            NetlinkError::Kernel(e) => write!(f, "netlink error {e}"),
            NetlinkError::NameNotUtf8 => f.write_str("interface name is not UTF-8"),
        }
    }
}

impl core::error::Error for NetlinkError {}

fn u16_at(b: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_ne_bytes(b.get(off..off + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_ne_bytes(b.get(off..off + 4)?.try_into().ok()?))
}

fn i32_at(b: &[u8], off: usize) -> Option<i32> {
    Some(i32::from_ne_bytes(b.get(off..off + 4)?.try_into().ok()?))
}

/// Walk a dump, handing each message's `(type, payload)` to `f`.
fn for_each_message<F>(dump: &[u8], mut f: F) -> Result<(), NetlinkError>
where
    F: FnMut(u16, &[u8]) -> Result<(), NetlinkError>,
{
    let mut offset = 0usize;
    while offset + NLMSGHDR_LEN <= dump.len() {
        let len = u32_at(dump, offset).ok_or(NetlinkError::Truncated)? as usize;
        let kind = u16_at(dump, offset + 4).ok_or(NetlinkError::Truncated)?;
        if len < NLMSGHDR_LEN || offset + len > dump.len() {
            return Err(NetlinkError::Truncated);
        }
        let payload = &dump[offset + NLMSGHDR_LEN..offset + len];
        match kind {
            NLMSG_DONE => return Ok(()),
            NLMSG_ERROR => {
                // `struct nlmsgerr { int error; struct nlmsghdr msg; }`.
                // Zero is an ACK, not a failure.
                let err = i32_at(payload, 0).ok_or(NetlinkError::Truncated)?;
                if err != 0 {
                    return Err(NetlinkError::Kernel(err));
                }
            }
            NLMSG_NOOP => {}
            other => f(other, payload)?,
        }
        offset += align4(len);
    }
    // A tail too short to be a header is a dump that was cut, not a dump that
    // ended: netlink terminates with `NLMSG_DONE` and nothing else. Returning
    // the messages read so far would be the short list P4(a) must never see.
    if offset != dump.len() {
        return Err(NetlinkError::Truncated);
    }
    Ok(())
}

/// `struct rtattr { u16 rta_len; u16 rta_type; }` + payload, `RTA_ALIGN`ed.
fn for_each_attribute<F>(mut body: &[u8], mut f: F) -> Result<(), NetlinkError>
where
    F: FnMut(u16, &[u8]) -> Result<(), NetlinkError>,
{
    while body.len() >= RTATTR_LEN {
        let len = u16_at(body, 0).ok_or(NetlinkError::Truncated)? as usize;
        let kind = u16_at(body, 2).ok_or(NetlinkError::Truncated)?;
        if len < RTATTR_LEN || len > body.len() {
            return Err(NetlinkError::Truncated);
        }
        f(kind, &body[RTATTR_LEN..len])?;
        let step = align4(len);
        if step >= body.len() {
            break;
        }
        body = &body[step..];
    }
    Ok(())
}

/// Parse an `RTM_GETLINK` dump into the interface set.
pub fn parse_links(dump: &[u8]) -> Result<Vec<Link>, NetlinkError> {
    let mut links = Vec::new();
    for_each_message(dump, |kind, payload| {
        if kind != RTM_NEWLINK {
            return Ok(());
        }
        if payload.len() < IFINFOMSG_LEN {
            return Err(NetlinkError::Truncated);
        }
        // `struct ifinfomsg { u8 family; u8 pad; u16 type; int index; u32 flags; u32 change; }`
        let index = i32_at(payload, 4).ok_or(NetlinkError::Truncated)? as u32;
        let flags = u32_at(payload, 8).ok_or(NetlinkError::Truncated)?;
        let mut name = String::new();
        for_each_attribute(&payload[IFINFOMSG_LEN..], |attr, value| {
            if attr == IFLA_IFNAME {
                let bytes = value.strip_suffix(b"\0").unwrap_or(value);
                name = core::str::from_utf8(bytes)
                    .map_err(|_| NetlinkError::NameNotUtf8)?
                    .to_string();
            }
            Ok(())
        })?;
        links.push(Link { index, name, flags });
        Ok(())
    })?;
    Ok(links)
}

/// Parse an `RTM_GETADDR` dump into the address set — **of any family**, which
/// is what P4(a) is written over.
///
/// `IFA_LOCAL` is preferred over `IFA_ADDRESS` where both are present: on a
/// point-to-point interface `IFA_ADDRESS` is the *peer's* address, so reading it
/// alone would judge the namespace by an address that is not in it.
pub fn parse_addrs(dump: &[u8]) -> Result<Vec<Addr>, NetlinkError> {
    let mut addrs = Vec::new();
    for_each_message(dump, |kind, payload| {
        if kind != RTM_NEWADDR {
            return Ok(());
        }
        if payload.len() < IFADDRMSG_LEN {
            return Err(NetlinkError::Truncated);
        }
        // `struct ifaddrmsg { u8 family; u8 prefixlen; u8 flags; u8 scope; u32 index; }`
        let family = payload[0];
        let prefix_len = payload[1];
        let index = u32_at(payload, 4).ok_or(NetlinkError::Truncated)?;
        let mut local: Option<Vec<u8>> = None;
        let mut address: Option<Vec<u8>> = None;
        for_each_attribute(&payload[IFADDRMSG_LEN..], |attr, value| {
            match attr {
                IFA_LOCAL => local = Some(value.to_vec()),
                IFA_ADDRESS => address = Some(value.to_vec()),
                _ => {}
            }
            Ok(())
        })?;
        if let Some(bytes) = local.or(address) {
            addrs.push(Addr {
                index,
                family,
                prefix_len,
                bytes,
            });
        }
        Ok(())
    })?;
    Ok(addrs)
}

/// Does this recv chunk contain the end of the dump?
pub fn chunk_ends_the_dump(chunk: &[u8]) -> bool {
    let mut offset = 0usize;
    while offset + NLMSGHDR_LEN <= chunk.len() {
        let Some(len) = u32_at(chunk, offset).map(|n| n as usize) else {
            return true;
        };
        let Some(kind) = u16_at(chunk, offset + 4) else {
            return true;
        };
        if len < NLMSGHDR_LEN {
            return true;
        }
        if kind == NLMSG_DONE || kind == NLMSG_ERROR {
            return true;
        }
        offset += align4(len);
    }
    false
}

fn header(len: usize, kind: u16, flags: u16, seq: u32) -> [u8; NLMSGHDR_LEN] {
    let mut h = [0u8; NLMSGHDR_LEN];
    h[0..4].copy_from_slice(&(len as u32).to_ne_bytes());
    h[4..6].copy_from_slice(&kind.to_ne_bytes());
    h[6..8].copy_from_slice(&flags.to_ne_bytes());
    h[8..12].copy_from_slice(&seq.to_ne_bytes());
    h
}

/// `RTM_GETLINK`, dump form — P4(a)'s first read.
pub fn get_link_request(seq: u32) -> Vec<u8> {
    let len = NLMSGHDR_LEN + IFINFOMSG_LEN;
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(&header(len, RTM_GETLINK, NLM_F_REQUEST | NLM_F_DUMP, seq));
    let mut body = [0u8; IFINFOMSG_LEN];
    body[0] = AF_UNSPEC;
    out.extend_from_slice(&body);
    out
}

/// `RTM_GETADDR`, dump form, `AF_UNSPEC` — *"no address of **any family**"* is
/// the pass condition, so the request must not name one.
pub fn get_addr_request(seq: u32) -> Vec<u8> {
    let len = NLMSGHDR_LEN + IFADDRMSG_LEN;
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(&header(len, RTM_GETADDR, NLM_F_REQUEST | NLM_F_DUMP, seq));
    let mut body = [0u8; IFADDRMSG_LEN];
    body[0] = AF_UNSPEC;
    out.extend_from_slice(&body);
    out
}

/// `RTM_NEWLINK` with `IFF_UP` — the collector bringing `lo` up, which is
/// prerequisite 5's second half: *"a network namespace … **with a loopback
/// device it can bring up**"*.
///
/// `ifi_change` is set to `IFF_UP` and no other bit, so this request turns one
/// flag on and cannot disturb another.
pub fn set_link_up_request(index: u32, seq: u32) -> Vec<u8> {
    let len = NLMSGHDR_LEN + IFINFOMSG_LEN;
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(&header(len, RTM_NEWLINK, NLM_F_REQUEST | NLM_F_ACK, seq));
    let mut body = [0u8; IFINFOMSG_LEN];
    body[0] = AF_UNSPEC;
    body[4..8].copy_from_slice(&(index as i32).to_ne_bytes());
    body[8..12].copy_from_slice(&IFF_UP.to_ne_bytes());
    body[12..16].copy_from_slice(&IFF_UP.to_ne_bytes());
    out.extend_from_slice(&body);
    out
}

#[cfg(test)]
pub(crate) mod build {
    //! Test-side encoders. They exist so the published measurement of RF §7.1
    //! can be reproduced **from bytes** rather than asserted as a list.
    use super::*;

    pub fn link_message(index: u32, name: &str, flags: u32) -> Vec<u8> {
        let attr_payload = {
            let mut v = name.as_bytes().to_vec();
            v.push(0);
            v
        };
        let attr_len = RTATTR_LEN + attr_payload.len();
        let len = NLMSGHDR_LEN + IFINFOMSG_LEN + align4(attr_len);
        let mut out = Vec::with_capacity(len);
        out.extend_from_slice(&header(len, RTM_NEWLINK, 0, 1));
        let mut body = [0u8; IFINFOMSG_LEN];
        body[4..8].copy_from_slice(&(index as i32).to_ne_bytes());
        body[8..12].copy_from_slice(&flags.to_ne_bytes());
        out.extend_from_slice(&body);
        out.extend_from_slice(&(attr_len as u16).to_ne_bytes());
        out.extend_from_slice(&IFLA_IFNAME.to_ne_bytes());
        out.extend_from_slice(&attr_payload);
        out.resize(len, 0);
        out
    }

    pub fn addr_message(index: u32, family: u8, prefix_len: u8, bytes: &[u8]) -> Vec<u8> {
        let attr_len = RTATTR_LEN + bytes.len();
        let len = NLMSGHDR_LEN + IFADDRMSG_LEN + align4(attr_len);
        let mut out = Vec::with_capacity(len);
        out.extend_from_slice(&header(len, RTM_NEWADDR, 0, 1));
        let mut body = [0u8; IFADDRMSG_LEN];
        body[0] = family;
        body[1] = prefix_len;
        body[4..8].copy_from_slice(&index.to_ne_bytes());
        out.extend_from_slice(&body);
        out.extend_from_slice(&(attr_len as u16).to_ne_bytes());
        out.extend_from_slice(&IFA_LOCAL.to_ne_bytes());
        out.extend_from_slice(bytes);
        out.resize(len, 0);
        out
    }

    pub fn done() -> Vec<u8> {
        header(NLMSGHDR_LEN, NLMSG_DONE, 0, 1).to_vec()
    }

    /// RF §7.1's measured namespace, device for device and flag for flag:
    /// *"Measured on Linux 6.11.11, a namespace made by nothing but
    /// `unshare(CLONE_NEWNET)` contains **ten** devices: `lo`, and `tunl0`,
    /// `gre0`, `gretap0`, `erspan0`, `ip_vti0`, `ip6_vti0`, `sit0`, `ip6tnl0`,
    /// `ip6gre0` — every one of the nine **down**."*
    pub fn the_measured_fresh_namespace() -> Vec<u8> {
        let devices: [(&str, u32); 10] = [
            // `IFF_LOOPBACK` alone, and **down**. A fresh namespace creates
            // loopback down; the collector brings it up, which is a step it
            // performs and can fail (RF §7.1, prerequisite 5).
            //
            // This value was `0x9` until 2026-08-27. `0x9` is the *job's* own
            // loopback, up, read through an inherited `sysfs` — the exact trap
            // RF §7.1 warns about, fallen into while measuring the thing it
            // warns about. Re-measured from a `sysfs` mounted inside the
            // namespace: `lo flags=0x8 operstate=down`.
            ("lo", 0x8),
            ("tunl0", 0x80),  // IFF_NOARP, down
            ("gre0", 0x80),
            ("gretap0", 0x1002), // IFF_BROADCAST | IFF_MULTICAST, down
            ("erspan0", 0x1002),
            ("ip_vti0", 0x80),
            ("ip6_vti0", 0x80),
            ("sit0", 0x80),
            ("ip6tnl0", 0x80),
            ("ip6gre0", 0x80),
        ];
        let mut dump = Vec::new();
        for (i, (name, flags)) in devices.iter().enumerate() {
            dump.extend_from_slice(&link_message(i as u32 + 1, name, *flags));
        }
        dump.extend_from_slice(&done());
        dump
    }

    /// What M1 actually hands a runner: the measured fresh namespace, with
    /// loopback **brought up** by the collector.
    ///
    /// This is the configuration P4 is measured against — *"the runner
    /// disposition … is the configuration the probe below is built from"* (RF
    /// §7.1) — and it differs from a bare `unshare(CLONE_NEWNET)` in exactly
    /// one bit, which is the bit prerequisite 5 is about.
    pub fn the_runner_disposition() -> Vec<u8> {
        let mut devices: Vec<(&str, u32)> = vec![
            ("lo", 0x9), // IFF_UP | IFF_LOOPBACK — the collector brought it up
        ];
        for (name, flags) in FRESH_TUNNEL_DEVICES {
            devices.push((name, flags));
        }
        let mut dump = Vec::new();
        for (i, (name, flags)) in devices.iter().enumerate() {
            dump.extend_from_slice(&link_message(i as u32 + 1, name, *flags));
        }
        dump.extend_from_slice(&done());
        dump
    }

    /// The nine devices the kernel instantiates in every new network namespace
    /// where the `ipip`, `gre`, `sit` and `ip6_tunnel` modules are loaded.
    const FRESH_TUNNEL_DEVICES: [(&str, u32); 9] = [
        ("tunl0", 0x80),     // IFF_NOARP, down
        ("gre0", 0x80),
        ("gretap0", 0x1002), // IFF_BROADCAST | IFF_MULTICAST, down
        ("erspan0", 0x1002),
        ("ip_vti0", 0x80),
        ("ip6_vti0", 0x80),
        ("sit0", 0x80),
        ("ip6tnl0", 0x80),
        ("ip6gre0", 0x80),
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RF §7.1's published measurement, reproduced from netlink bytes: ten
    /// devices, **all ten down**, loopback included.
    #[test]
    fn the_measured_fresh_namespace_is_ten_devices_all_of_them_down() {
        let links = parse_links(&build::the_measured_fresh_namespace()).unwrap();
        let names: Vec<&str> = links.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "lo", "tunl0", "gre0", "gretap0", "erspan0", "ip_vti0", "ip6_vti0", "sit0",
                "ip6tnl0", "ip6gre0"
            ]
        );
        assert_eq!(links.len(), 10);
        assert_eq!(
            links.iter().filter(|l| l.is_up()).count(),
            0,
            "a fresh namespace has no interface up at all — loopback included"
        );
        assert!(links[0].is_loopback() && !links[0].is_up());
        for down in &links[1..] {
            assert!(!down.is_up(), "{} must be down", down.name);
            assert!(!down.is_loopback(), "{} is not loopback", down.name);
        }
    }

    /// The address set of that namespace, verbatim from the measurement:
    /// *"IPv4 addresses in this netns: (none) / IPv6 addresses in this netns:
    /// (none)"* before `lo` is configured, and loopback's alone after.
    #[test]
    fn loopbacks_two_addresses_are_the_only_two_p4a_admits() {
        let mut dump = build::addr_message(1, AF_INET, 8, &[127, 0, 0, 1]);
        let mut v6 = [0u8; 16];
        v6[15] = 1;
        dump.extend_from_slice(&build::addr_message(1, AF_INET6, 128, &v6));
        dump.extend_from_slice(&build::done());

        let addrs = parse_addrs(&dump).unwrap();
        assert_eq!(addrs.len(), 2);
        assert!(addrs[0].is_the_loopback_v4());
        assert!(addrs[1].is_the_loopback_v6());

        // 127.0.0.2/8 is loopback traffic and is still not one of the two the
        // spec names, so the parser must not fold it into either.
        let other = Addr {
            index: 1,
            family: AF_INET,
            prefix_len: 8,
            bytes: vec![127, 0, 0, 2],
        };
        assert!(!other.is_the_loopback_v4());
    }

    /// A dump the parser cannot fully account for must be an error, never a
    /// short list: P4(a) passes on an *absence*, so silently dropping the tail
    /// of a dump is the way to pass without measuring anything.
    #[test]
    fn a_truncated_dump_is_an_error_and_never_a_short_list() {
        let full = build::the_measured_fresh_namespace();
        let cut = &full[..full.len() - 6];
        assert_eq!(parse_links(cut), Err(NetlinkError::Truncated));

        let mut bad = build::link_message(1, "lo", 0x9);
        bad[0..4].copy_from_slice(&4u32.to_ne_bytes()); // len below the header
        assert_eq!(parse_links(&bad), Err(NetlinkError::Truncated));
    }

    /// `NLMSG_ERROR` with a non-zero errno stops the parse. A zero errno is an
    /// ACK and is not an error.
    #[test]
    fn a_kernel_error_message_stops_the_parse() {
        let mut dump = build::link_message(1, "lo", 0x9);
        let mut err = header(NLMSGHDR_LEN + 4, NLMSG_ERROR, 0, 1).to_vec();
        err.extend_from_slice(&(-13i32).to_ne_bytes()); // EACCES
        dump.extend_from_slice(&err);
        assert_eq!(parse_links(&dump), Err(NetlinkError::Kernel(-13)));
    }

    /// The two dump requests, and the one write. `RTM_GETADDR` must ask for
    /// `AF_UNSPEC`: P4(a) is written over "no address of **any family**", and a
    /// request naming `AF_INET` would pass a namespace holding an IPv6 route.
    #[test]
    fn the_addr_dump_asks_for_every_family() {
        let req = get_addr_request(7);
        assert_eq!(req.len(), NLMSGHDR_LEN + IFADDRMSG_LEN);
        assert_eq!(u16_at(&req, 4), Some(RTM_GETADDR));
        assert_eq!(u16_at(&req, 6), Some(NLM_F_REQUEST | NLM_F_DUMP));
        assert_eq!(req[NLMSGHDR_LEN], AF_UNSPEC);

        let req = get_link_request(7);
        assert_eq!(u16_at(&req, 4), Some(RTM_GETLINK));
        assert_eq!(req.len(), NLMSGHDR_LEN + IFINFOMSG_LEN);
    }

    /// Bringing `lo` up must change `IFF_UP` and nothing else: `ifi_change` is
    /// the mask the kernel applies, and an all-ones mask would clear every flag
    /// the request did not set.
    #[test]
    fn bringing_loopback_up_changes_only_the_up_flag() {
        let req = set_link_up_request(1, 3);
        let body = &req[NLMSGHDR_LEN..];
        assert_eq!(u32_at(body, 8), Some(IFF_UP), "ifi_flags");
        assert_eq!(u32_at(body, 12), Some(IFF_UP), "ifi_change");
    }

    /// The recv loop must stop on `NLMSG_DONE`, or a dump read would block
    /// forever inside the probe and the deadline would be the only thing that
    /// ended it.
    #[test]
    fn a_chunk_carrying_done_ends_the_dump() {
        assert!(chunk_ends_the_dump(&build::the_measured_fresh_namespace()));
        assert!(!chunk_ends_the_dump(&build::link_message(1, "lo", 0x9)));
    }
}
