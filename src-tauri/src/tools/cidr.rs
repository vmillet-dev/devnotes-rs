//! A CIDR block: what it holds, what it is, whether an address falls in it, and how it splits.
//! Both families are computed on `u128`, an IPv4 address in its low 32 bits.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use ipnet::{IpNet, ipv4_mask_to_prefix, ipv6_mask_to_prefix};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Past this, a split says how many more there are rather than listing them.
pub const SUBNETS_SHOWN: usize = 256;
const SPLIT_CHOICES: u8 = 3;
/// 2¹²⁸, the addresses of `::/0`: one more than a `u128` holds.
const ALL_IPV6: &str = "340282366920938463463374607431768211456";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum IpFamily {
    V4,
    V6,
}

impl IpFamily {
    fn bits(self) -> u8 {
        match self {
            Self::V4 => 32,
            Self::V6 => 128,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CidrRequest {
    pub network: String,
    /// `None` reads the family from the text.
    pub family: Option<IpFamily>,
    pub member: String,
    pub split: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CidrProblem {
    Unreadable,
    PrefixTooLong,
    MaskWithHoles,
    WrongFamily,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum BlockKind {
    ThisNetwork,
    Private,
    SharedAddressSpace,
    Loopback,
    LinkLocal,
    Documentation,
    Benchmarking,
    Multicast,
    Reserved,
    Broadcast,
    Public,
    Unspecified,
    Ipv4Mapped,
    Nat64,
    UniqueLocal,
    Global,
    /// Wider than a special range it holds: no one sentence describes it.
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CidrBlock {
    pub family: IpFamily,
    pub cidr: String,
    pub prefix: u8,
    pub host_bits: u8,
    /// The address had host bits set: `cidr` is its network.
    pub normalised: bool,
    /// The network address in binary, cut where the host bits start.
    pub binary_network: String,
    pub binary_host: String,
    pub kind: BlockKind,
    /// The special range the block lies in, when it names one: `10.0.0.0/8`.
    pub range: Option<String>,
    pub network: String,
    /// IPv4 only.
    pub mask: Option<String>,
    pub inverse_mask: Option<String>,
    pub first: String,
    pub last: String,
    /// IPv4 from `/30` up: `/31` and `/32` have none, nor IPv6.
    pub broadcast: Option<String>,
    /// Decimal digits: an IPv6 count passes what a JSON number holds.
    pub addresses: String,
    pub usable: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CidrMembership {
    Empty,
    Unreadable,
    Outside,
    /// Counted from the network address, which is host 0.
    Inside {
        host: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Subnet {
    pub cidr: String,
    pub usable: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CidrSplit {
    pub prefix: u8,
    pub subnets: Vec<Subnet>,
    /// How many past `SUBNETS_SHOWN`, in decimal digits.
    pub more: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CidrAnswer {
    pub block: Option<CidrBlock>,
    pub problem: Option<CidrProblem>,
    pub membership: CidrMembership,
    /// The prefixes offered as chips: the next three.
    pub split_choices: Vec<u8>,
    pub split: Option<CidrSplit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Net {
    family: IpFamily,
    network: u128,
    prefix: u8,
}

impl Net {
    fn bits(self) -> u8 {
        self.family.bits()
    }

    fn host_bits(self) -> u8 {
        self.bits() - self.prefix
    }

    fn host_mask(self) -> u128 {
        ones(self.host_bits())
    }

    fn net_mask(self) -> u128 {
        ones(self.bits()) & !self.host_mask()
    }

    fn last(self) -> u128 {
        self.network | self.host_mask()
    }

    fn contains(self, address: u128) -> bool {
        address & self.net_mask() == self.network
    }

    /// `other` lies wholly inside `self`.
    fn holds(self, other: Self) -> bool {
        self.family == other.family && other.prefix >= self.prefix && self.contains(other.network)
    }

    fn cidr(self) -> String {
        format!("{}/{}", address(self.family, self.network), self.prefix)
    }
}

fn ones(count: u8) -> u128 {
    if count >= 128 {
        u128::MAX
    } else {
        (1 << count) - 1
    }
}

fn address(family: IpFamily, value: u128) -> String {
    match family {
        IpFamily::V4 => Ipv4Addr::from(u32::try_from(value).unwrap_or(u32::MAX)).to_string(),
        IpFamily::V6 => Ipv6Addr::from(value).to_string(),
    }
}

fn split_ip(ip: IpAddr) -> (IpFamily, u128) {
    match ip {
        IpAddr::V4(v4) => (IpFamily::V4, u128::from(u32::from(v4))),
        IpAddr::V6(v6) => (IpFamily::V6, u128::from(v6)),
    }
}

pub fn describe(request: &CidrRequest) -> CidrAnswer {
    let text = request.network.trim();
    if text.is_empty() {
        return CidrAnswer {
            block: None,
            problem: None,
            membership: CidrMembership::Empty,
            split_choices: Vec::new(),
            split: None,
        };
    }
    let read = parse(text).and_then(|(net, address)| forced(net, address, request.family));
    let (net, typed) = match read {
        Ok(read) => read,
        Err(problem) => {
            return CidrAnswer {
                block: None,
                problem: Some(problem),
                membership: CidrMembership::Empty,
                split_choices: Vec::new(),
                split: None,
            };
        }
    };
    let split_choices: Vec<u8> = (net.prefix + 1..=net.bits())
        .take(usize::from(SPLIT_CHOICES))
        .collect();
    let split = request
        .split
        .filter(|prefix| (net.prefix + 1..=net.bits()).contains(prefix))
        .or_else(|| split_choices.first().copied())
        .map(|prefix| split(net, prefix));
    CidrAnswer {
        block: Some(block(net, typed != net.network)),
        problem: None,
        membership: membership(net, &request.member),
        split_choices,
        split,
    }
}

/// The block and the address as typed: `10.24.8.5/21` is `10.24.8.0/21`, host bits set.
fn parse(text: &str) -> Result<(Net, u128), CidrProblem> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    let (ip, prefix) = match parts.as_slice() {
        [ip, mask] => {
            let ip = IpAddr::from_str(ip).map_err(|_| CidrProblem::Unreadable)?;
            let mask = IpAddr::from_str(mask).map_err(|_| CidrProblem::Unreadable)?;
            let prefix = match (ip, mask) {
                (IpAddr::V4(_), IpAddr::V4(mask)) => ipv4_mask_to_prefix(mask),
                (IpAddr::V6(_), IpAddr::V6(mask)) => ipv6_mask_to_prefix(mask),
                _ => return Err(CidrProblem::WrongFamily),
            }
            .map_err(|_| CidrProblem::MaskWithHoles)?;
            (ip, prefix)
        }
        [single] => {
            if let Some((ip, prefix)) = single.split_once('/') {
                let ip = IpAddr::from_str(ip).map_err(|_| CidrProblem::Unreadable)?;
                let prefix: u8 = prefix.parse().map_err(|_| CidrProblem::Unreadable)?;
                IpNet::new(ip, prefix).map_err(|_| CidrProblem::PrefixTooLong)?;
                (ip, prefix)
            } else {
                let ip = IpAddr::from_str(single).map_err(|_| CidrProblem::Unreadable)?;
                (ip, split_ip(ip).0.bits())
            }
        }
        _ => return Err(CidrProblem::Unreadable),
    };
    let (family, typed) = split_ip(ip);
    let mut net = Net {
        family,
        network: typed,
        prefix,
    };
    net.network = typed & net.net_mask();
    Ok((net, typed))
}

/// A forced family reads an IPv4 block as its IPv4-mapped IPv6 one, and back.
fn forced(net: Net, typed: u128, family: Option<IpFamily>) -> Result<(Net, u128), CidrProblem> {
    const MAPPED: u128 = 0xffff << 32;
    match (net.family, family) {
        (_, None) | (IpFamily::V4, Some(IpFamily::V4)) | (IpFamily::V6, Some(IpFamily::V6)) => {
            Ok((net, typed))
        }
        (IpFamily::V4, Some(IpFamily::V6)) => Ok((
            Net {
                family: IpFamily::V6,
                network: MAPPED | net.network,
                prefix: net.prefix + 96,
            },
            MAPPED | typed,
        )),
        (IpFamily::V6, Some(IpFamily::V4)) => {
            let mapped = Net {
                family: IpFamily::V6,
                network: MAPPED,
                prefix: 96,
            };
            if mapped.holds(net) {
                Ok((
                    Net {
                        family: IpFamily::V4,
                        network: net.network & ones(32),
                        prefix: net.prefix - 96,
                    },
                    typed & ones(32),
                ))
            } else {
                Err(CidrProblem::WrongFamily)
            }
        }
    }
}

fn block(net: Net, normalised: bool) -> CidrBlock {
    let (binary_network, binary_host) = binary(net);
    let (kind, range) = classify(net);
    let v4 = net.family == IpFamily::V4;
    let usable_hosts = v4 && net.host_bits() >= 2;
    CidrBlock {
        family: net.family,
        cidr: net.cidr(),
        prefix: net.prefix,
        host_bits: net.host_bits(),
        normalised,
        binary_network,
        binary_host,
        kind,
        range: range.map(Net::cidr),
        network: address(net.family, net.network),
        mask: v4.then(|| address(net.family, net.net_mask())),
        inverse_mask: v4.then(|| address(net.family, net.host_mask())),
        first: address(net.family, net.network + u128::from(usable_hosts)),
        last: address(net.family, net.last() - u128::from(usable_hosts)),
        broadcast: usable_hosts.then(|| address(net.family, net.last())),
        addresses: count(net.host_bits()),
        usable: usable(net),
    }
}

/// `2^bits`, in digits: `::/0` holds one more than a `u128`.
fn count(bits: u8) -> String {
    if bits >= 128 {
        ALL_IPV6.to_owned()
    } else {
        (1_u128 << bits).to_string()
    }
}

/// The network and broadcast addresses are not hosts, but on a `/31` (RFC 3021) and a `/32`.
fn usable(net: Net) -> String {
    match (net.family, net.host_bits()) {
        (IpFamily::V4, bits @ 2..) => ((1_u128 << bits) - 2).to_string(),
        (_, bits) => count(bits),
    }
}

fn binary(net: Net) -> (String, String) {
    let (group, separator) = match net.family {
        IpFamily::V4 => (8, '.'),
        IpFamily::V6 => (16, ':'),
    };
    let bits = net.bits();
    let mut text = String::new();
    let mut cut = None;
    for index in 0..bits {
        if index == net.prefix {
            cut = Some(text.len());
        }
        if index > 0 && index % group == 0 {
            text.push(separator);
        }
        let bit = (net.network >> (bits - 1 - index)) & 1;
        text.push(if bit == 1 { '1' } else { '0' });
    }
    let cut = cut.unwrap_or(text.len());
    let host = text.split_off(cut);
    (text, host)
}

/// The IANA special-purpose registries, the most specific range first.
fn special(family: IpFamily) -> Vec<(Net, BlockKind)> {
    let v4 = |a: u8, b: u8, c: u8, d: u8, prefix, kind| {
        (
            Net {
                family: IpFamily::V4,
                network: u128::from(u32::from(Ipv4Addr::new(a, b, c, d))),
                prefix,
            },
            kind,
        )
    };
    let v6 = |network: u128, prefix, kind| {
        (
            Net {
                family: IpFamily::V6,
                network,
                prefix,
            },
            kind,
        )
    };
    let mut table = match family {
        IpFamily::V4 => vec![
            v4(0, 0, 0, 0, 8, BlockKind::ThisNetwork),
            v4(10, 0, 0, 0, 8, BlockKind::Private),
            v4(100, 64, 0, 0, 10, BlockKind::SharedAddressSpace),
            v4(127, 0, 0, 0, 8, BlockKind::Loopback),
            v4(169, 254, 0, 0, 16, BlockKind::LinkLocal),
            v4(172, 16, 0, 0, 12, BlockKind::Private),
            v4(192, 0, 2, 0, 24, BlockKind::Documentation),
            v4(192, 168, 0, 0, 16, BlockKind::Private),
            v4(198, 18, 0, 0, 15, BlockKind::Benchmarking),
            v4(198, 51, 100, 0, 24, BlockKind::Documentation),
            v4(203, 0, 113, 0, 24, BlockKind::Documentation),
            v4(224, 0, 0, 0, 4, BlockKind::Multicast),
            v4(240, 0, 0, 0, 4, BlockKind::Reserved),
            v4(255, 255, 255, 255, 32, BlockKind::Broadcast),
        ],
        IpFamily::V6 => vec![
            v6(0, 128, BlockKind::Unspecified),
            v6(1, 128, BlockKind::Loopback),
            v6(0xffff << 32, 96, BlockKind::Ipv4Mapped),
            v6(0x0064_ff9b << 96, 96, BlockKind::Nat64),
            v6(0x2001_0db8 << 96, 32, BlockKind::Documentation),
            v6(0x2 << 124, 3, BlockKind::Global),
            v6(0xfc << 120, 7, BlockKind::UniqueLocal),
            v6(0xfe80 << 112, 10, BlockKind::LinkLocal),
            v6(0xff << 120, 8, BlockKind::Multicast),
        ],
    };
    table.sort_by_key(|(range, _)| std::cmp::Reverse(range.prefix));
    table
}

fn classify(net: Net) -> (BlockKind, Option<Net>) {
    let table = special(net.family);
    if let Some((range, kind)) = table.iter().find(|(range, _)| range.holds(net)) {
        return (*kind, Some(*range));
    }
    if table.iter().any(|(range, _)| net.holds(*range)) {
        return (BlockKind::Mixed, None);
    }
    match net.family {
        IpFamily::V4 => (BlockKind::Public, None),
        IpFamily::V6 => (BlockKind::Reserved, None),
    }
}

fn membership(net: Net, member: &str) -> CidrMembership {
    let member = member.trim();
    if member.is_empty() {
        return CidrMembership::Empty;
    }
    let Ok(ip) = IpAddr::from_str(member) else {
        return CidrMembership::Unreadable;
    };
    let (family, value) = split_ip(ip);
    if family == net.family && net.contains(value) {
        CidrMembership::Inside {
            host: (value - net.network).to_string(),
        }
    } else {
        CidrMembership::Outside
    }
}

fn split(net: Net, prefix: u8) -> CidrSplit {
    let step_bits = net.bits() - prefix;
    let count_bits = prefix - net.prefix;
    let subnets = (0..)
        .take_while(|index: &u128| count_bits >= 128 || *index < (1_u128 << count_bits))
        .take(SUBNETS_SHOWN)
        .map(|index| {
            let subnet = Net {
                family: net.family,
                network: net.network + (index << step_bits),
                prefix,
            };
            Subnet {
                cidr: subnet.cidr(),
                usable: usable(subnet),
            }
        })
        .collect();
    CidrSplit {
        prefix,
        subnets,
        more: more_than_shown(count_bits),
    }
}

fn more_than_shown(count_bits: u8) -> String {
    const SHOWN: u128 = SUBNETS_SHOWN as u128;
    if count_bits >= 128 {
        // 2¹²⁸ − 256, past a `u128` like `ALL_IPV6`.
        return "340282366920938463463374607431768211200".to_owned();
    }
    (1_u128 << count_bits).saturating_sub(SHOWN).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(network: &str) -> CidrAnswer {
        describe(&CidrRequest {
            network: network.to_owned(),
            family: None,
            member: String::new(),
            split: None,
        })
    }

    fn block_of(network: &str) -> CidrBlock {
        ask(network)
            .block
            .unwrap_or_else(|| panic!("{network} read as a block"))
    }

    fn member(network: &str, member: &str) -> CidrMembership {
        describe(&CidrRequest {
            network: network.to_owned(),
            family: None,
            member: member.to_owned(),
            split: None,
        })
        .membership
    }

    fn split_into(network: &str, prefix: u8) -> CidrSplit {
        describe(&CidrRequest {
            network: network.to_owned(),
            family: None,
            member: String::new(),
            split: Some(prefix),
        })
        .split
        .unwrap()
    }

    #[test]
    fn a_slash_21_holds_2046_hosts_between_its_network_and_broadcast() {
        let block = block_of("10.24.8.0/21");

        assert_eq!(block.cidr, "10.24.8.0/21");
        assert!(!block.normalised);
        assert_eq!((block.prefix, block.host_bits), (21, 11));
        assert_eq!(block.mask.as_deref(), Some("255.255.248.0"));
        assert_eq!(block.inverse_mask.as_deref(), Some("0.0.7.255"));
        assert_eq!(
            (block.first.as_str(), block.last.as_str()),
            ("10.24.8.1", "10.24.15.254")
        );
        assert_eq!(block.broadcast.as_deref(), Some("10.24.15.255"));
        assert_eq!(
            (block.usable.as_str(), block.addresses.as_str()),
            ("2046", "2048")
        );
        assert_eq!(
            (block.binary_network.as_str(), block.binary_host.as_str()),
            ("00001010.00011000.00001", "000.00000000")
        );
    }

    #[test]
    fn host_bits_set_are_cleared_and_said() {
        let block = block_of("10.24.8.5/21");

        assert_eq!(block.cidr, "10.24.8.0/21");
        assert!(block.normalised);
    }

    #[test]
    fn a_mask_reads_as_its_prefix_and_a_mask_with_holes_is_refused() {
        assert_eq!(block_of("192.168.1.0 255.255.255.0").cidr, "192.168.1.0/24");
        assert_eq!(
            ask("10.0.0.0 255.0.255.0").problem,
            Some(CidrProblem::MaskWithHoles)
        );
        assert_eq!(
            ask("10.0.0.0 ffff::").problem,
            Some(CidrProblem::WrongFamily)
        );
    }

    #[test]
    fn what_cannot_be_read_says_why() {
        assert_eq!(ask("10.0.0.0/33").problem, Some(CidrProblem::PrefixTooLong));
        assert_eq!(ask("10.0.0/8").problem, Some(CidrProblem::Unreadable));
        assert_eq!(ask("exemple.fr").problem, Some(CidrProblem::Unreadable));
        assert_eq!(ask("   "), ask(""));
        assert_eq!(ask("").block, None);
    }

    #[test]
    fn slash_0_and_slash_8_count_their_addresses() {
        let all = block_of("0.0.0.0/0");
        assert_eq!(
            (all.addresses.as_str(), all.usable.as_str()),
            ("4294967296", "4294967294")
        );
        assert_eq!(all.kind, BlockKind::Mixed);

        let eight = block_of("10.0.0.0/8");
        assert_eq!(eight.last, "10.255.255.254");
        assert_eq!(eight.usable, "16777214");
    }

    #[test]
    fn a_slash_31_and_a_slash_32_have_no_network_nor_broadcast_to_take_away() {
        let pair = block_of("192.0.2.4/31");
        assert_eq!(
            (pair.first.as_str(), pair.last.as_str()),
            ("192.0.2.4", "192.0.2.5")
        );
        assert_eq!((pair.broadcast, pair.usable.as_str()), (None, "2"));

        let one = block_of("192.0.2.4");
        assert_eq!(one.cidr, "192.0.2.4/32");
        assert_eq!(
            (one.first.as_str(), one.last.as_str()),
            ("192.0.2.4", "192.0.2.4")
        );
        assert_eq!((one.broadcast, one.usable.as_str()), (None, "1"));
        assert_eq!(one.binary_host, "");
    }

    #[test]
    fn a_block_is_named_after_the_special_range_it_lies_in() {
        let kind = |network| {
            let block = block_of(network);
            (block.kind, block.range)
        };

        assert_eq!(
            kind("10.24.8.0/21"),
            (BlockKind::Private, Some("10.0.0.0/8".to_owned()))
        );
        assert_eq!(
            kind("172.20.0.0/16"),
            (BlockKind::Private, Some("172.16.0.0/12".to_owned()))
        );
        assert_eq!(kind("100.64.0.0/10").0, BlockKind::SharedAddressSpace);
        assert_eq!(kind("127.0.0.1").0, BlockKind::Loopback);
        assert_eq!(kind("169.254.10.0/24").0, BlockKind::LinkLocal);
        assert_eq!(kind("203.0.113.0/24").0, BlockKind::Documentation);
        assert_eq!(kind("239.1.1.1").0, BlockKind::Multicast);
        assert_eq!(kind("255.255.255.255").0, BlockKind::Broadcast);
        assert_eq!(kind("8.8.8.0/24"), (BlockKind::Public, None));
        assert_eq!(kind("10.0.0.0/7"), (BlockKind::Mixed, None));
        assert_eq!(kind("2001:db8::/32").0, BlockKind::Documentation);
        assert_eq!(kind("2a01:e0a::/32").0, BlockKind::Global);
        assert_eq!(kind("fd12:3456::/48").0, BlockKind::UniqueLocal);
        assert_eq!(kind("fe80::1").0, BlockKind::LinkLocal);
        assert_eq!(kind("::1").0, BlockKind::Loopback);
        assert_eq!(kind("::ffff:10.0.0.1").0, BlockKind::Ipv4Mapped);
    }

    #[test]
    fn an_ipv6_slash_64_has_no_broadcast_and_counts_every_address() {
        let block = block_of("2001:db8:0:1::5/64");

        assert_eq!(block.cidr, "2001:db8:0:1::/64");
        assert!(block.normalised);
        assert_eq!(
            (block.mask, block.inverse_mask, block.broadcast),
            (None, None, None)
        );
        assert_eq!(block.first, "2001:db8:0:1::");
        assert_eq!(block.last, "2001:db8:0:1:ffff:ffff:ffff:ffff");
        assert_eq!(block.addresses, "18446744073709551616");
        assert_eq!(block.usable, block.addresses);
        assert_eq!(block.binary_network.split(':').count(), 4);
        assert!(block.binary_host.starts_with(':'));
    }

    #[test]
    fn an_ipv6_slash_128_is_one_address_and_slash_0_all_of_them_exactly() {
        let one = block_of("2001:db8::1/128");
        assert_eq!(
            (one.addresses.as_str(), one.first.as_str()),
            ("1", "2001:db8::1")
        );

        assert_eq!(block_of("::/0").addresses, ALL_IPV6);
    }

    #[test]
    fn a_forced_family_reads_ipv4_as_mapped_ipv6_and_back() {
        let forced = |network: &str, family| {
            describe(&CidrRequest {
                network: network.to_owned(),
                family: Some(family),
                member: String::new(),
                split: None,
            })
        };

        let mapped = forced("10.0.0.0/8", IpFamily::V6).block.unwrap();
        assert_eq!(mapped.cidr, "::ffff:10.0.0.0/104");
        assert_eq!(mapped.kind, BlockKind::Ipv4Mapped);

        let back = forced("::ffff:10.0.0.0/104", IpFamily::V4).block.unwrap();
        assert_eq!(back.cidr, "10.0.0.0/8");
        assert_eq!(
            forced("2001:db8::/32", IpFamily::V4).problem,
            Some(CidrProblem::WrongFamily)
        );
    }

    #[test]
    fn an_address_is_inside_at_its_host_number_and_outside_past_the_edges() {
        assert_eq!(
            member("10.24.8.0/21", "10.24.12.40"),
            CidrMembership::Inside {
                host: "1064".to_owned()
            }
        );
        assert_eq!(
            member("10.24.8.0/21", "10.24.8.0"),
            CidrMembership::Inside {
                host: "0".to_owned()
            }
        );
        assert_eq!(
            member("10.24.8.0/21", "10.24.15.255"),
            CidrMembership::Inside {
                host: "2047".to_owned()
            }
        );
        assert_eq!(
            member("10.24.8.0/21", "10.24.16.0"),
            CidrMembership::Outside
        );
        assert_eq!(
            member("10.24.8.0/21", "10.24.7.255"),
            CidrMembership::Outside
        );
        assert_eq!(
            member("10.24.8.0/21", "2001:db8::1"),
            CidrMembership::Outside
        );
        assert_eq!(member("10.24.8.0/21", "10.24"), CidrMembership::Unreadable);
        assert_eq!(member("10.24.8.0/21", " "), CidrMembership::Empty);
    }

    #[test]
    fn a_block_splits_into_the_next_three_prefixes_the_first_by_default() {
        let answer = ask("10.24.8.0/21");

        assert_eq!(answer.split_choices, vec![22, 23, 24]);
        let split = answer.split.unwrap();
        assert_eq!(split.prefix, 22);
        assert_eq!(
            split.subnets,
            vec![
                Subnet {
                    cidr: "10.24.8.0/22".to_owned(),
                    usable: "1022".to_owned()
                },
                Subnet {
                    cidr: "10.24.12.0/22".to_owned(),
                    usable: "1022".to_owned()
                },
            ]
        );
        assert_eq!(split.more, "0");
        assert_eq!(
            split_into("10.24.8.0/21", 23).subnets[3].cidr,
            "10.24.14.0/23"
        );
        assert_eq!(ask("10.0.0.1/31").split_choices, vec![32]);
        assert!(ask("10.0.0.1/32").split.is_none());
    }

    #[test]
    fn a_split_lists_the_first_256_and_counts_the_rest() {
        let split = split_into("10.0.0.0/8", 24);

        assert_eq!(split.subnets.len(), SUBNETS_SHOWN);
        assert_eq!(split.subnets[255].cidr, "10.0.255.0/24");
        assert_eq!(split.more, "65280");

        let all = split_into("::/0", 128);
        assert_eq!(all.subnets[1].cidr, "::1/128");
        assert_eq!(all.more, "340282366920938463463374607431768211200");
    }

    #[test]
    fn a_prefix_out_of_reach_falls_back_to_the_first_choice() {
        assert_eq!(split_into("10.24.8.0/21", 20).prefix, 22);
        assert_eq!(split_into("10.24.8.0/21", 33).prefix, 22);
    }
}
