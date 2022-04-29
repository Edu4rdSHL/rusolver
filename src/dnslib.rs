use {
    crate::structs::{DomainData, LibOptions},
    futures::stream::{self, StreamExt},
    std::{
        collections::{HashMap, HashSet},
        net::SocketAddr,
    },
    trust_dns_resolver::{
        config::{NameServerConfig, NameServerConfigGroup, Protocol, ResolverConfig, ResolverOpts},
        lookup::{Ipv4Lookup, Lookup},
        name_server::{GenericConnection, GenericConnectionProvider, TokioRuntime},
        proto::{rr::RecordType, xfer::DnsRequestOptions},
        AsyncResolver, TokioAsyncResolver,
    },
};

pub fn return_tokio_asyncresolver(
    nameserver_ips: HashSet<String>,
    options: ResolverOpts,
) -> AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>> {
    let mut name_servers = NameServerConfigGroup::with_capacity(nameserver_ips.len() * 2);

    name_servers.extend(nameserver_ips.into_iter().flat_map(|server| {
        let socket_addr = SocketAddr::V4(match server.parse() {
            Ok(a) => a,
            Err(e) => unreachable!(
                "Error parsing the server {}, only IPv4 are allowed. Error: {}",
                server, e
            ),
        });

        std::iter::once(NameServerConfig {
            socket_addr,
            protocol: Protocol::Udp,
            tls_dns_name: None,
            trust_nx_responses: false,
        })
        .chain(std::iter::once(NameServerConfig {
            socket_addr,
            protocol: Protocol::Tcp,
            tls_dns_name: None,
            trust_nx_responses: false,
        }))
    }));

    TokioAsyncResolver::tokio(
        ResolverConfig::from_parts(None, vec![], name_servers),
        options,
    )
    .unwrap()
}

pub async fn return_hosts_data(options: &LibOptions) -> HashMap<String, DomainData> {
    let threads = if options.hosts.len() < options.threads {
        options.hosts.len()
    } else {
        options.threads
    };

    stream::iter(options.hosts.clone().into_iter().map(|host| {
        let lookup_host = host.trim_end_matches('.').to_owned() + ".";

        let resolver_fut = options.resolvers.ipv4_lookup(lookup_host.clone());
        let trustable_resolver_fut = options.trustable_resolver.ipv4_lookup(lookup_host);
        let wildcard_ips = options.wildcard_ips.clone();

        let mut domain_data = DomainData::default();

        let mut ip_lookup = Option::<Ipv4Lookup>::None;

        async move {
            if let Ok(ip) = resolver_fut.await {
                if options.disable_double_check {
                    ip_lookup = Some(ip);
                } else if let Ok(ip) = trustable_resolver_fut.await {
                    ip_lookup = Some(ip);
                }
            }

            if let Some(ip_lookup) = ip_lookup {
                for ip in ip_lookup.iter() {
                    domain_data.ipv4_addresses.insert(ip.to_string());
                }
            }

            domain_data.is_wildcard = domain_data
                .ipv4_addresses
                .iter()
                .all(|ip| wildcard_ips.contains(ip));

            if !options.quiet_flag {
                if options.show_ip_address && !domain_data.is_wildcard {
                    println!("{};{:?}", host, domain_data.ipv4_addresses);
                } else if !domain_data.is_wildcard {
                    println!("{}", host)
                }
            }

            (host, domain_data)
        }
    }))
    .buffer_unordered(threads)
    .collect::<HashMap<String, DomainData>>()
    .await
}

// Used internally for now
pub async fn return_cname_data(
    hosts: HashSet<String>,
    resolver: AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>,
    trustable_resolver: AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>,
    disable_double_check: bool,
    mut threads: usize,
) -> HashMap<String, DomainData> {
    if hosts.len() < threads {
        threads = hosts.len();
    }

    let request_options = DnsRequestOptions::default();
    let record_type = RecordType::CNAME;

    stream::iter(hosts)
        .map(|host| {
            let lookup_host = host.trim_end_matches('.').to_owned() + ".";
            let resolver_fut = resolver.lookup(lookup_host.clone(), record_type, request_options);
            let trustable_resolver_fut =
                trustable_resolver.lookup(lookup_host, record_type, request_options);

            let mut domain_data = DomainData::default();

            let mut cname_lookup = Option::<Lookup>::None;

            async move {
                if let Ok(lookup) = resolver_fut.await {
                    if disable_double_check {
                        cname_lookup = Some(lookup);
                    } else if let Ok(lookup) = trustable_resolver_fut.await {
                        cname_lookup = Some(lookup);
                    }
                }

                if let Some(lookup) = cname_lookup {
                    for record in lookup.iter() {
                        if let Some(cname) = record.as_cname() {
                            domain_data.cname = cname.to_string();
                        }
                    }
                }

                (host, domain_data)
            }
        })
        .buffer_unordered(threads)
        .collect::<HashMap<String, DomainData>>()
        .await
}
