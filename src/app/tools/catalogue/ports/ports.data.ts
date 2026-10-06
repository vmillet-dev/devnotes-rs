export const PORT_RANGES = ['well-known', 'registered', 'dynamic'] as const;
export type PortRange = (typeof PORT_RANGES)[number];

export type Transport = 'tcp' | 'udp' | 'both';

export interface Port {
  readonly number: number;
  readonly transport: Transport;
  /** A protocol or a product's name, in no language. */
  readonly service: string;
}

export interface PortWords {
  /** Keyed by the port number. */
  readonly ports: Readonly<Record<string, string>>;
}

/** IANA's three ranges. */
export function portRange(port: number): PortRange {
  if (port <= 1023) return 'well-known';
  return port <= 49151 ? 'registered' : 'dynamic';
}

export const PORTS: readonly Port[] = [
  { number: 20, transport: 'tcp', service: 'FTP (data)' },
  { number: 21, transport: 'tcp', service: 'FTP' },
  { number: 22, transport: 'tcp', service: 'SSH' },
  { number: 23, transport: 'tcp', service: 'Telnet' },
  { number: 25, transport: 'tcp', service: 'SMTP' },
  { number: 53, transport: 'both', service: 'DNS' },
  { number: 67, transport: 'udp', service: 'DHCP (server)' },
  { number: 68, transport: 'udp', service: 'DHCP (client)' },
  { number: 69, transport: 'udp', service: 'TFTP' },
  { number: 80, transport: 'tcp', service: 'HTTP' },
  { number: 110, transport: 'tcp', service: 'POP3' },
  { number: 123, transport: 'udp', service: 'NTP' },
  { number: 139, transport: 'tcp', service: 'NetBIOS' },
  { number: 143, transport: 'tcp', service: 'IMAP' },
  { number: 161, transport: 'udp', service: 'SNMP' },
  { number: 179, transport: 'tcp', service: 'BGP' },
  { number: 389, transport: 'both', service: 'LDAP' },
  { number: 443, transport: 'both', service: 'HTTPS' },
  { number: 445, transport: 'tcp', service: 'SMB' },
  { number: 465, transport: 'tcp', service: 'SMTPS' },
  { number: 514, transport: 'udp', service: 'Syslog' },
  { number: 587, transport: 'tcp', service: 'SMTP (submission)' },
  { number: 636, transport: 'tcp', service: 'LDAPS' },
  { number: 853, transport: 'both', service: 'DNS over TLS' },
  { number: 993, transport: 'tcp', service: 'IMAPS' },
  { number: 995, transport: 'tcp', service: 'POP3S' },

  { number: 1080, transport: 'tcp', service: 'SOCKS' },
  { number: 1194, transport: 'both', service: 'OpenVPN' },
  { number: 1433, transport: 'tcp', service: 'SQL Server' },
  { number: 1521, transport: 'tcp', service: 'Oracle Database' },
  { number: 1883, transport: 'tcp', service: 'MQTT' },
  { number: 2049, transport: 'both', service: 'NFS' },
  { number: 2181, transport: 'tcp', service: 'ZooKeeper' },
  { number: 2375, transport: 'tcp', service: 'Docker API' },
  { number: 2376, transport: 'tcp', service: 'Docker API (TLS)' },
  { number: 2379, transport: 'tcp', service: 'etcd' },
  { number: 3000, transport: 'tcp', service: 'Node.js, Grafana, Rails' },
  { number: 3306, transport: 'tcp', service: 'MySQL, MariaDB' },
  { number: 3389, transport: 'both', service: 'RDP' },
  { number: 3478, transport: 'both', service: 'STUN, TURN' },
  { number: 4200, transport: 'tcp', service: 'Angular (ng serve)' },
  { number: 5000, transport: 'tcp', service: 'Flask' },
  { number: 5173, transport: 'tcp', service: 'Vite' },
  { number: 5222, transport: 'tcp', service: 'XMPP' },
  { number: 5353, transport: 'udp', service: 'mDNS' },
  { number: 5432, transport: 'tcp', service: 'PostgreSQL' },
  { number: 5601, transport: 'tcp', service: 'Kibana' },
  { number: 5672, transport: 'tcp', service: 'AMQP (RabbitMQ)' },
  { number: 5900, transport: 'tcp', service: 'VNC' },
  { number: 6379, transport: 'tcp', service: 'Redis' },
  { number: 6443, transport: 'tcp', service: 'Kubernetes API' },
  { number: 8000, transport: 'tcp', service: 'Django, http.server' },
  { number: 8080, transport: 'tcp', service: 'HTTP (alternative)' },
  { number: 8443, transport: 'tcp', service: 'HTTPS (alternative)' },
  { number: 8888, transport: 'tcp', service: 'Jupyter' },
  { number: 9000, transport: 'tcp', service: 'PHP-FPM, SonarQube, MinIO' },
  { number: 9090, transport: 'tcp', service: 'Prometheus' },
  { number: 9092, transport: 'tcp', service: 'Kafka' },
  { number: 9200, transport: 'tcp', service: 'Elasticsearch' },
  { number: 9418, transport: 'tcp', service: 'Git' },
  { number: 11211, transport: 'tcp', service: 'Memcached' },
  { number: 15672, transport: 'tcp', service: 'RabbitMQ (management)' },
  { number: 27017, transport: 'tcp', service: 'MongoDB' },

  { number: 51820, transport: 'udp', service: 'WireGuard' },
];
