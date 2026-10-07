//! The network address written otherwise: as one integer, in full, and in the reverse DNS tree.

use serde::Serialize;
use specta::Type;

use super::{IpFamily, Net};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ReverseDns {
    /// One address: the name its PTR record goes under.
    Record { name: String },
    /// The zones the block covers whole.
    Zones {
        first: String,
        last: String,
        count: u32,
    },
    /// Smaller than the zone it lies in: delegated through RFC 2317's aliases.
    Classless { zone: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CidrForms {
    pub integer: String,
    /// IPv4 in hexadecimal, IPv6 with every zero written out.
    pub expanded: String,
    pub reverse: ReverseDns,
}

pub(super) fn forms(net: Net) -> CidrForms {
    CidrForms {
        integer: net.network.to_string(),
        expanded: expanded(net),
        reverse: reverse(net),
    }
}

fn expanded(net: Net) -> String {
    match net.family {
        IpFamily::V4 => format!("0x{:08X}", net.network),
        IpFamily::V6 => (0..8)
            .map(|group| format!("{:04x}", (net.network >> (112 - 16 * group)) & 0xffff))
            .collect::<Vec<_>>()
            .join(":"),
    }
}

/// An octet a label in IPv4, a nibble in IPv6.
fn label_bits(family: IpFamily) -> u8 {
    match family {
        IpFamily::V4 => 8,
        IpFamily::V6 => 4,
    }
}

fn reverse(net: Net) -> ReverseDns {
    let unit = label_bits(net.family);
    let bits = net.bits();
    if net.prefix == bits {
        return ReverseDns::Record {
            name: name(net.family, net.network, bits),
        };
    }
    let zone_prefix = net.prefix.div_ceil(unit).max(1) * unit;
    if net.family == IpFamily::V4 && zone_prefix > 24 {
        return ReverseDns::Classless {
            zone: name(net.family, net.network, 24),
        };
    }
    let last_zone = net.last() & !super::ones(bits - zone_prefix);
    ReverseDns::Zones {
        first: name(net.family, net.network, zone_prefix),
        last: name(net.family, last_zone, zone_prefix),
        count: 1 << (zone_prefix - net.prefix),
    }
}

/// The name of the zone holding the first `prefix` bits of `value`, its labels reversed.
fn name(family: IpFamily, value: u128, prefix: u8) -> String {
    let unit = label_bits(family);
    let bits = family.bits();
    let mut labels: Vec<String> = (0..prefix / unit)
        .map(|index| {
            let label = (value >> (bits - unit * (index + 1))) & super::ones(unit);
            match family {
                IpFamily::V4 => label.to_string(),
                IpFamily::V6 => format!("{label:x}"),
            }
        })
        .collect();
    labels.reverse();
    let suffix = match family {
        IpFamily::V4 => "in-addr.arpa",
        IpFamily::V6 => "ip6.arpa",
    };
    labels.push(suffix.to_owned());
    labels.join(".")
}

#[cfg(test)]
mod tests {
    use super::super::{CidrRequest, describe};
    use super::*;

    fn forms_of(network: &str) -> CidrForms {
        describe(&CidrRequest {
            network: network.to_owned(),
            ..CidrRequest::default()
        })
        .block
        .unwrap()
        .forms
    }

    fn zones(first: &str, last: &str, count: u32) -> ReverseDns {
        ReverseDns::Zones {
            first: first.to_owned(),
            last: last.to_owned(),
            count,
        }
    }

    #[test]
    fn an_ipv4_network_reads_as_one_integer_and_in_hexadecimal() {
        let forms = forms_of("10.24.8.0/21");

        assert_eq!(forms.integer, "169347072");
        assert_eq!(forms.expanded, "0x0A180800");
    }

    #[test]
    fn an_ipv6_network_is_written_out_in_full() {
        let forms = forms_of("2001:db8::/32");

        assert_eq!(forms.expanded, "2001:0db8:0000:0000:0000:0000:0000:0000");
        assert_eq!(forms.integer, "42540766411282592856903984951653826560");
    }

    #[test]
    fn a_block_on_an_octet_boundary_is_one_zone_and_a_slash_21_eight() {
        assert_eq!(
            forms_of("10.24.0.0/16").reverse,
            zones("24.10.in-addr.arpa", "24.10.in-addr.arpa", 1)
        );
        assert_eq!(
            forms_of("10.24.8.0/21").reverse,
            zones("8.24.10.in-addr.arpa", "15.24.10.in-addr.arpa", 8)
        );
        assert_eq!(
            forms_of("0.0.0.0/0").reverse,
            zones("0.in-addr.arpa", "255.in-addr.arpa", 256)
        );
    }

    #[test]
    fn past_a_slash_24_the_zone_is_delegated_and_one_address_is_a_record() {
        assert_eq!(
            forms_of("192.0.2.64/26").reverse,
            ReverseDns::Classless {
                zone: "2.0.192.in-addr.arpa".to_owned()
            }
        );
        assert_eq!(
            forms_of("192.0.2.10").reverse,
            ReverseDns::Record {
                name: "10.2.0.192.in-addr.arpa".to_owned()
            }
        );
    }

    #[test]
    fn ipv6_zones_follow_the_nibbles() {
        assert_eq!(
            forms_of("2001:db8::/32").reverse,
            zones("8.b.d.0.1.0.0.2.ip6.arpa", "8.b.d.0.1.0.0.2.ip6.arpa", 1)
        );
        assert_eq!(
            forms_of("2001:db8::/30").reverse,
            zones("8.b.d.0.1.0.0.2.ip6.arpa", "b.b.d.0.1.0.0.2.ip6.arpa", 4)
        );
        let ReverseDns::Record { name } = forms_of("2001:db8::1").reverse else {
            panic!("one address is one record");
        };
        assert!(name.starts_with("1.0.0.0."));
        assert_eq!(name.split('.').count(), 34);
    }
}
