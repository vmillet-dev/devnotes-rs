//! A list of blocks, addresses and ranges brought to the fewest blocks covering exactly the
//! same addresses, and to the one block holding them all, as a route summarises them.

use std::net::IpAddr;
use std::str::FromStr;

use serde::Serialize;
use specta::Type;

use super::{IpFamily, Net, blocks_between, ones, parse, split_ip};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FamilySummary {
    pub family: IpFamily,
    pub blocks: Vec<String>,
    /// The smallest single block holding every address listed.
    pub supernet: String,
    /// What the supernet holds beyond the list, in digits.
    pub extra: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CidrSummary {
    /// IPv4 first, then IPv6, each when the list holds some.
    pub families: Vec<FamilySummary>,
    pub unreadable: Vec<String>,
}

pub(super) fn summarise(text: &str) -> Option<CidrSummary> {
    let mut ranges: Vec<(IpFamily, u128, u128)> = Vec::new();
    let mut unreadable = Vec::new();
    for entry in text
        .split(['\n', ',', ';'])
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        match range(entry) {
            Some(range) => ranges.push(range),
            None => unreadable.push(entry.to_owned()),
        }
    }
    if ranges.is_empty() && unreadable.is_empty() {
        return None;
    }
    let families = [IpFamily::V4, IpFamily::V6]
        .into_iter()
        .filter_map(|family| {
            let mut own: Vec<(u128, u128)> = ranges
                .iter()
                .filter(|range| range.0 == family)
                .map(|range| (range.1, range.2))
                .collect();
            (!own.is_empty()).then(|| family_summary(family, &mut own))
        })
        .collect();
    Some(CidrSummary {
        families,
        unreadable,
    })
}

/// `10.0.0.0/24`, `10.0.0.0 255.255.255.0`, one address, or `first - last`.
fn range(entry: &str) -> Option<(IpFamily, u128, u128)> {
    if let Some((first, last)) = entry.split_once('-') {
        let (family, first) = split_ip(IpAddr::from_str(first.trim()).ok()?);
        let (other, last) = split_ip(IpAddr::from_str(last.trim()).ok()?);
        return (family == other).then(|| (family, first.min(last), first.max(last)));
    }
    let (net, _) = parse(entry).ok()?;
    Some((net.family, net.network, net.last()))
}

fn family_summary(family: IpFamily, ranges: &mut [(u128, u128)]) -> FamilySummary {
    ranges.sort_unstable();
    let mut merged: Vec<(u128, u128)> = Vec::new();
    for &(start, end) in ranges.iter() {
        match merged.last_mut() {
            Some(last) if start <= last.1.saturating_add(1) => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    let low = merged[0].0;
    let high = merged[merged.len() - 1].1;
    let bits = family.bits();
    let differing = low ^ high;
    let prefix = if differing == 0 {
        bits
    } else {
        u8::try_from(differing.leading_zeros()).unwrap_or(0) - (128 - bits)
    };
    let mut supernet = Net {
        family,
        network: low,
        prefix,
    };
    supernet.network &= supernet.net_mask();
    // One less than the addresses covered, which cannot pass a `u128` when the list is all of IPv6.
    let covered_less_one = merged
        .iter()
        .map(|(start, end)| end - start)
        .fold(0_u128, u128::saturating_add)
        .saturating_add(merged.len() as u128 - 1);
    FamilySummary {
        family,
        blocks: merged
            .iter()
            .flat_map(|(start, end)| blocks_between(family, *start, *end))
            .map(Net::cidr)
            .collect(),
        supernet: supernet.cidr(),
        extra: (ones(supernet.host_bits()) - covered_less_one).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{CidrRequest, describe};
    use super::*;

    fn summary_of(list: &str) -> CidrSummary {
        describe(&CidrRequest {
            list: list.to_owned(),
            ..CidrRequest::default()
        })
        .summary
        .unwrap()
    }

    #[test]
    fn neighbours_merge_and_a_range_becomes_the_blocks_it_spans() {
        let summary = summary_of(
            "10.24.9.0/24\n10.24.8.0/24\n10.24.10.0/23, 10.24.12.0/24\n10.24.13.20 - 10.24.13.5",
        );

        assert!(summary.unreadable.is_empty());
        assert_eq!(
            summary.families,
            vec![FamilySummary {
                family: IpFamily::V4,
                blocks: vec![
                    "10.24.8.0/22".to_owned(),
                    "10.24.12.0/24".to_owned(),
                    "10.24.13.5/32".to_owned(),
                    "10.24.13.6/31".to_owned(),
                    "10.24.13.8/29".to_owned(),
                    "10.24.13.16/30".to_owned(),
                    "10.24.13.20/32".to_owned(),
                ],
                supernet: "10.24.8.0/21".to_owned(),
                extra: "752".to_owned(),
            }]
        );
    }

    #[test]
    fn overlaps_count_once_and_an_exact_list_has_nothing_extra() {
        let summary = summary_of("192.168.0.0/23\n192.168.1.0/24\n192.168.1.7");

        let v4 = &summary.families[0];
        assert_eq!(v4.blocks, vec!["192.168.0.0/23"]);
        assert_eq!(
            (v4.supernet.as_str(), v4.extra.as_str()),
            ("192.168.0.0/23", "0")
        );
    }

    #[test]
    fn each_family_is_summarised_apart_and_what_cannot_be_read_is_named() {
        let summary = summary_of("2001:db8::/48\n2001:db8:1::/48\n10.0.0.1\nici\n10.0.0.1 - ::1");

        assert_eq!(
            summary
                .families
                .iter()
                .map(|family| (family.family, family.supernet.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (IpFamily::V4, "10.0.0.1/32"),
                (IpFamily::V6, "2001:db8::/47"),
            ]
        );
        assert_eq!(summary.unreadable, vec!["ici", "10.0.0.1 - ::1"]);
    }

    #[test]
    fn all_of_ipv6_is_one_block_with_nothing_extra() {
        let summary = summary_of(":: - ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff");

        let v6 = &summary.families[0];
        assert_eq!(v6.blocks, vec!["::/0"]);
        assert_eq!(v6.extra, "0");
    }

    #[test]
    fn a_blank_list_is_none() {
        assert!(
            describe(&CidrRequest {
                list: " , \n".to_owned(),
                ..CidrRequest::default()
            })
            .summary
            .is_none()
        );
    }
}
