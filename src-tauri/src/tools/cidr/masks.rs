//! Every prefix of the family beside its mask and its size: the table a block is read against.

use serde::Serialize;
use specta::Type;

use super::{IpFamily, Net, address, count, usable};

/// IPv6's prefixes as they are handed out: by the nibble down to a LAN's `/64`, then the few below.
const V6_PREFIXES: [u8; 21] = [
    16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 80, 96, 112, 120, 124, 126, 127, 128,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MaskRow {
    pub prefix: u8,
    pub host_bits: u8,
    /// IPv4 only.
    pub mask: Option<String>,
    pub wildcard: Option<String>,
    pub addresses: String,
    pub usable: String,
    /// IPv6 only, up to `/64`: the `/64` networks, a LAN each, it holds.
    pub networks64: Option<String>,
}

pub(super) fn masks(net: Net) -> Vec<MaskRow> {
    let mut prefixes = match net.family {
        IpFamily::V4 => (0..=32).collect(),
        IpFamily::V6 => V6_PREFIXES.to_vec(),
    };
    if !prefixes.contains(&net.prefix) {
        prefixes.push(net.prefix);
        prefixes.sort_unstable();
    }
    prefixes
        .into_iter()
        .map(|prefix| {
            row(Net {
                family: net.family,
                network: 0,
                prefix,
            })
        })
        .collect()
}

fn row(net: Net) -> MaskRow {
    let v4 = net.family == IpFamily::V4;
    MaskRow {
        prefix: net.prefix,
        host_bits: net.host_bits(),
        mask: v4.then(|| address(net.family, net.net_mask())),
        wildcard: v4.then(|| address(net.family, net.host_mask())),
        addresses: count(net.host_bits()),
        usable: usable(net),
        networks64: (!v4 && net.prefix <= 64).then(|| count(64 - net.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{CidrRequest, describe};
    use super::*;

    fn masks_of(network: &str) -> Vec<MaskRow> {
        describe(&CidrRequest {
            network: network.to_owned(),
            ..CidrRequest::default()
        })
        .masks
    }

    #[test]
    fn ipv4_lists_every_prefix_with_its_mask_and_its_hosts() {
        let masks = masks_of("10.24.8.0/21");

        assert_eq!(masks.len(), 33);
        assert_eq!(
            masks[21],
            MaskRow {
                prefix: 21,
                host_bits: 11,
                mask: Some("255.255.248.0".to_owned()),
                wildcard: Some("0.0.7.255".to_owned()),
                addresses: "2048".to_owned(),
                usable: "2046".to_owned(),
                networks64: None,
            }
        );
        assert_eq!(masks[0].mask.as_deref(), Some("0.0.0.0"));
        assert_eq!(masks[31].usable, "2");
    }

    #[test]
    fn ipv6_lists_the_prefixes_handed_out_and_the_one_typed() {
        let masks = masks_of("2001:db8::/47");
        let prefixes: Vec<u8> = masks.iter().map(|row| row.prefix).collect();

        assert_eq!(prefixes.len(), V6_PREFIXES.len() + 1);
        assert!(prefixes.windows(2).all(|pair| pair[0] < pair[1]));
        let forty_eight = masks.iter().find(|row| row.prefix == 48).unwrap();
        assert_eq!(forty_eight.networks64.as_deref(), Some("65536"));
        assert_eq!(forty_eight.mask, None);
        let last = masks.last().unwrap();
        assert_eq!((last.prefix, last.networks64.as_ref()), (128, None));
    }

    #[test]
    fn no_block_no_table() {
        assert!(masks_of("").is_empty());
        assert!(masks_of("10.0.0.0/33").is_empty());
    }
}
