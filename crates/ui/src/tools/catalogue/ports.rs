//! The ports worth knowing: the facts here, their notes in the reference's words.

use i18n::{Language, keys};
use serde_json::json;

use super::reference::{Reference, Table, columns_of, groups_of, word};
use crate::tools::Tool;
use crate::tools::ui::{Cell, ReferenceRow};

const RANGES: [&str; 3] = ["well-known", "registered", "dynamic"];

/// The number, its transport, and a protocol or a product's name, in no language.
const PORTS: [(u32, &str, &str); 64] = [
    (20, "tcp", "FTP (data)"),
    (21, "tcp", "FTP"),
    (22, "tcp", "SSH"),
    (23, "tcp", "Telnet"),
    (25, "tcp", "SMTP"),
    (53, "both", "DNS"),
    (67, "udp", "DHCP (server)"),
    (68, "udp", "DHCP (client)"),
    (69, "udp", "TFTP"),
    (80, "tcp", "HTTP"),
    (110, "tcp", "POP3"),
    (123, "udp", "NTP"),
    (139, "tcp", "NetBIOS"),
    (143, "tcp", "IMAP"),
    (161, "udp", "SNMP"),
    (179, "tcp", "BGP"),
    (389, "both", "LDAP"),
    (443, "both", "HTTPS"),
    (445, "tcp", "SMB"),
    (465, "tcp", "SMTPS"),
    (514, "udp", "Syslog"),
    (587, "tcp", "SMTP (submission)"),
    (636, "tcp", "LDAPS"),
    (853, "both", "DNS over TLS"),
    (993, "tcp", "IMAPS"),
    (995, "tcp", "POP3S"),
    (1080, "tcp", "SOCKS"),
    (1194, "both", "OpenVPN"),
    (1433, "tcp", "SQL Server"),
    (1521, "tcp", "Oracle Database"),
    (1883, "tcp", "MQTT"),
    (2049, "both", "NFS"),
    (2181, "tcp", "ZooKeeper"),
    (2375, "tcp", "Docker API"),
    (2376, "tcp", "Docker API (TLS)"),
    (2379, "tcp", "etcd"),
    (3000, "tcp", "Node.js, Grafana, Rails"),
    (3306, "tcp", "MySQL, MariaDB"),
    (3389, "both", "RDP"),
    (3478, "both", "STUN, TURN"),
    (4200, "tcp", "Angular (ng serve)"),
    (5000, "tcp", "Flask"),
    (5173, "tcp", "Vite"),
    (5222, "tcp", "XMPP"),
    (5353, "udp", "mDNS"),
    (5432, "tcp", "PostgreSQL"),
    (5601, "tcp", "Kibana"),
    (5672, "tcp", "AMQP (RabbitMQ)"),
    (5900, "tcp", "VNC"),
    (6379, "tcp", "Redis"),
    (6443, "tcp", "Kubernetes API"),
    (8000, "tcp", "Django, http.server"),
    (8080, "tcp", "HTTP (alternative)"),
    (8443, "tcp", "HTTPS (alternative)"),
    (8888, "tcp", "Jupyter"),
    (9000, "tcp", "PHP-FPM, SonarQube, MinIO"),
    (9090, "tcp", "Prometheus"),
    (9092, "tcp", "Kafka"),
    (9200, "tcp", "Elasticsearch"),
    (9418, "tcp", "Git"),
    (11211, "tcp", "Memcached"),
    (15672, "tcp", "RabbitMQ (management)"),
    (27017, "tcp", "MongoDB"),
    (51820, "udp", "WireGuard"),
];

/// IANA's three ranges.
fn range_of(port: u32) -> &'static str {
    match port {
        0..=1023 => "well-known",
        1024..=49151 => "registered",
        _ => "dynamic",
    }
}

/// A number is read as the start of a port, so that « 54 » finds 5432 and not 1521's note.
fn matches_port(row: &ReferenceRow, query: &str) -> bool {
    let wanted = query.trim();
    if !wanted.is_empty() && wanted.bytes().all(|it| it.is_ascii_digit()) {
        return row.key.starts_with(wanted);
    }
    row.matches(&crate::api::tools::fold(wanted))
}

fn row((port, transport, service): (u32, &str, &str), language: Language) -> ReferenceRow {
    let range = range_of(port);
    let note = word(language, &format!("tools.ports.words.ports.{port}"));
    let carried = word(
        language,
        &format!(
            "tools.ports.transports.{}",
            json!(transport).as_str().unwrap_or("")
        ),
    );
    let heading = word(language, &format!("tools.ports.ranges.{range}"));
    let cells = vec![
        Cell::mono(port.to_string()),
        Cell::text(carried),
        Cell::text(service),
        Cell::text(note.clone()),
    ];
    let key = port.to_string();
    let title = format!("{key} {service}");
    ReferenceRow::new(key.clone(), range, cells)
        .searched_by([key.as_str(), service, &note, &heading])
        .found_as(title, note)
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: PORTS.iter().map(|it| row(*it, language)).collect(),
        groups: groups_of(language, "tools.ports.ranges", &RANGES),
        columns: columns_of(
            language,
            "tools.ports.columns",
            &[
                ("port", Some(0.11)),
                ("transport", Some(0.16)),
                ("service", Some(0.3)),
                ("note", None),
            ],
        ),
        caption: keys::tools::ports::NAME,
        placeholder: keys::tools::ports::PLACEHOLDER,
        card_width: 520.0,
        matches: Some(matches_port),
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Reference::of(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_is_the_start_of_a_port() {
        let postgres = row((5432, "tcp", "PostgreSQL"), Language::En);

        assert!(matches_port(&postgres, "54"));
        assert!(!matches_port(&postgres, "432"));
    }

    #[test]
    fn the_three_ranges_are_iana_s() {
        assert_eq!(range_of(443), "well-known");
        assert_eq!(range_of(8080), "registered");
        assert_eq!(range_of(50_000), "dynamic");
    }
}
