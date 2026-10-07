//! An address plan: needs of so many hosts each, placed in the block the largest first, so that
//! every subnet starts on its own boundary and nothing is lost between two of them.

use serde::Serialize;
use specta::Type;

use super::{Net, address, blocks_between, usable, usable_count};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PlanLine {
    Placed {
        name: String,
        hosts: String,
        cidr: String,
        usable: String,
        first: String,
        last: String,
    },
    /// What is left of the block is too small for it.
    NoRoom { name: String, hosts: String },
    /// No host count on that line.
    Unreadable { text: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CidrPlan {
    /// In address order, which is the largest need first; the unreadable lines last.
    pub lines: Vec<PlanLine>,
    /// What is left of the block, as the fewest blocks.
    pub free: Vec<String>,
}

struct Need {
    name: String,
    hosts: u128,
}

pub(super) fn plan(net: Net, text: &str) -> Option<CidrPlan> {
    let mut needs = Vec::new();
    let mut unreadable = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        match need(line) {
            Some(need) => needs.push(need),
            None => unreadable.push(PlanLine::Unreadable {
                text: line.to_owned(),
            }),
        }
    }
    if needs.is_empty() && unreadable.is_empty() {
        return None;
    }
    // Stable: equal needs keep the order they were typed in.
    needs.sort_by_key(|need| std::cmp::Reverse(need.hosts));

    let mut lines = Vec::with_capacity(needs.len() + unreadable.len());
    let mut cursor = Some(net.network);
    for need in needs {
        let hosts = need.hosts.to_string();
        let placed = prefix_for(net, need.hosts).and_then(|prefix| {
            let subnet = Net {
                family: net.family,
                network: cursor?,
                prefix,
            };
            (subnet.last() <= net.last()).then_some(subnet)
        });
        let Some(subnet) = placed else {
            lines.push(PlanLine::NoRoom {
                name: need.name,
                hosts,
            });
            continue;
        };
        let (first, last) = super::host_range(subnet);
        lines.push(PlanLine::Placed {
            name: need.name,
            hosts,
            cidr: subnet.cidr(),
            usable: usable(subnet),
            first: address(net.family, first),
            last: address(net.family, last),
        });
        cursor = subnet
            .last()
            .checked_add(1)
            .filter(|next| *next <= net.last());
    }
    lines.extend(unreadable);
    let free = cursor
        .map(|start| blocks_between(net.family, start, net.last()))
        .unwrap_or_default();
    Some(CidrPlan {
        lines,
        free: free.into_iter().map(Net::cidr).collect(),
    })
}

/// `Bureaux 500`, `Bureaux : 500` or `500 Bureaux`; the count last when both ends are numbers.
fn need(line: &str) -> Option<Need> {
    let count = |token: &str| token.parse::<u128>().ok().filter(|hosts| *hosts > 0);
    let trimmed = |name: &str| {
        name.trim()
            .trim_end_matches([':', '=', ',', ';', '-'])
            .trim()
            .to_owned()
    };
    if let Some((name, last)) = line.rsplit_once(char::is_whitespace)
        && let Some(hosts) = count(last)
    {
        return Some(Need {
            name: trimmed(name),
            hosts,
        });
    }
    if let Some((first, name)) = line.split_once(char::is_whitespace)
        && let Some(hosts) = count(first)
    {
        return Some(Need {
            name: trimmed(name),
            hosts,
        });
    }
    count(line).map(|hosts| Need {
        name: String::new(),
        hosts,
    })
}

/// The smallest subnet of the block's family holding that many hosts, if the block can.
fn prefix_for(net: Net, hosts: u128) -> Option<u8> {
    (net.prefix..=net.bits()).rev().find(|prefix| {
        usable_count(Net {
            family: net.family,
            network: 0,
            prefix: *prefix,
        }) >= hosts
    })
}

#[cfg(test)]
mod tests {
    use super::super::{CidrRequest, describe};
    use super::*;

    fn plan_of(network: &str, plan: &str) -> CidrPlan {
        describe(&CidrRequest {
            network: network.to_owned(),
            plan: plan.to_owned(),
            ..CidrRequest::default()
        })
        .plan
        .unwrap()
    }

    fn placed(line: &PlanLine) -> (&str, &str) {
        match line {
            PlanLine::Placed { name, cidr, .. } => (name, cidr),
            other => panic!("{other:?} was not placed"),
        }
    }

    #[test]
    fn needs_are_placed_the_largest_first_each_on_its_boundary() {
        let plan = plan_of(
            "10.24.8.0/21",
            "Lien WAN 2\nBureaux 500\nServeurs : 60\n\n120 Wi-Fi invités\nImprimantes 12",
        );

        let placed: Vec<_> = plan.lines.iter().map(placed).collect();
        assert_eq!(
            placed,
            vec![
                ("Bureaux", "10.24.8.0/23"),
                ("Wi-Fi invités", "10.24.10.0/25"),
                ("Serveurs", "10.24.10.128/26"),
                ("Imprimantes", "10.24.10.192/28"),
                ("Lien WAN", "10.24.10.208/31"),
            ]
        );
        assert_eq!(
            plan.free,
            vec![
                "10.24.10.210/31",
                "10.24.10.212/30",
                "10.24.10.216/29",
                "10.24.10.224/27",
                "10.24.11.0/24",
                "10.24.12.0/22",
            ]
        );
        let PlanLine::Placed {
            usable,
            first,
            last,
            ..
        } = &plan.lines[0]
        else {
            unreachable!()
        };
        assert_eq!(
            (usable.as_str(), first.as_str(), last.as_str()),
            ("510", "10.24.8.1", "10.24.9.254")
        );
    }

    #[test]
    fn a_need_past_what_is_left_has_no_room_and_the_smaller_ones_still_fit() {
        let plan = plan_of("192.168.1.0/24", "A 200\nB 100\nC 20");

        assert_eq!(placed(&plan.lines[0]), ("A", "192.168.1.0/24"));
        assert_eq!(
            plan.lines[1],
            PlanLine::NoRoom {
                name: "B".to_owned(),
                hosts: "100".to_owned()
            }
        );
        assert!(matches!(plan.lines[2], PlanLine::NoRoom { .. }));
        assert!(plan.free.is_empty());

        let plan = plan_of("192.168.1.0/25", "A 200\nB 20");
        assert!(matches!(plan.lines[0], PlanLine::NoRoom { .. }));
        assert_eq!(placed(&plan.lines[1]), ("B", "192.168.1.0/27"));
    }

    #[test]
    fn a_line_without_a_count_is_said_and_a_blank_plan_is_none() {
        let plan = plan_of("10.0.0.0/8", "Bureaux\n40");

        assert_eq!(placed(&plan.lines[0]), ("", "10.0.0.0/26"));
        assert_eq!(
            plan.lines[1],
            PlanLine::Unreadable {
                text: "Bureaux".to_owned()
            }
        );
        assert!(
            describe(&CidrRequest {
                network: "10.0.0.0/8".to_owned(),
                plan: " \n ".to_owned(),
                ..CidrRequest::default()
            })
            .plan
            .is_none()
        );
    }

    #[test]
    fn ipv6_places_whole_networks_up_to_the_last_address() {
        let plan = plan_of("2001:db8::/126", "a 2\nb 2");

        let placed: Vec<_> = plan.lines.iter().map(placed).collect();
        assert_eq!(
            placed,
            vec![("a", "2001:db8::/127"), ("b", "2001:db8::2/127")]
        );
        assert!(plan.free.is_empty());
    }
}
