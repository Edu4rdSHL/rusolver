use trust_dns_resolver::proto::{rr::RecordType, xfer::DnsRequestOptions};

use {
    crate::structs::DomainData,
    futures::stream::{self, StreamExt},
    std::collections::{HashMap, HashSet},
    trust_dns_resolver::config::ResolverOpts,
};

use {
    std::net::SocketAddr,
    trust_dns_resolver::{
        config::{NameServerConfig, NameServerConfigGroup, Protocol, ResolverConfig},
        name_server::{GenericConnection, GenericConnectionProvider, TokioRuntime},
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

#[allow(clippy::too_many_arguments)]
pub async fn return_hosts_data(
    hosts: HashSet<String>,
    resolver: AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>,
    trustable_resolver: AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>,
    wildcard_ips: HashSet<String>,
    disable_double_check: bool,
    mut threads: usize,
    show_ip_addresses: bool,
    quiet_flag: bool,
) -> HashMap<String, DomainData> {
    if hosts.len() < threads {
        threads = hosts.len();
    }

    stream::iter(hosts)
        .map(|host| {
            let resolver_fut = resolver.ipv4_lookup(host.trim_end_matches('.').to_owned() + ".");
            let trustable_resolver_fut =
                trustable_resolver.ipv4_lookup(host.trim_end_matches('.').to_owned() + ".");
            let wildcard_ips = wildcard_ips.clone();

            let mut domain_data = DomainData::default();

            async move {
                if let Ok(ip) = resolver_fut.await {
                    if disable_double_check {
                        domain_data.ipv4_addresses = ip
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect::<HashSet<String>>();
                    } else if let Ok(ip) = trustable_resolver_fut.await {
                        domain_data.ipv4_addresses = ip
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect::<HashSet<String>>();
                    }
                }
                domain_data.is_wildcard = domain_data
                    .ipv4_addresses
                    .iter()
                    .all(|ip| wildcard_ips.contains(ip));

                if !quiet_flag {
                    if show_ip_addresses && !domain_data.is_wildcard {
                        println!("{};{:?}", host, domain_data.ipv4_addresses);
                    } else if !domain_data.is_wildcard {
                        println!("{}", host)
                    }
                }

                (host, domain_data)
            }
        })
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
            let resolver_fut = resolver.lookup(
                host.trim_end_matches('.').to_owned() + ".",
                record_type,
                request_options,
            );
            let trustable_resolver_fut = trustable_resolver.lookup(
                host.trim_end_matches('.').to_owned() + ".",
                record_type,
                request_options,
            );

            let mut domain_data = DomainData::default();

            async move {
                if let Ok(lookup) = resolver_fut.await {
                    if disable_double_check {
                        domain_data.cname = lookup
                            .iter()
                            .filter_map(|rdata| rdata.as_cname())
                            .map(|name| {
                                let name = name.to_string();
                                name[..name.len() - 1].to_owned()
                            })
                            .collect();
                    } else if let Ok(lookup) = trustable_resolver_fut.await {
                        domain_data.cname = lookup
                            .iter()
                            .filter_map(|rdata| rdata.as_cname())
                            .map(|name| {
                                let name = name.to_string();
                                name[..name.len() - 1].to_owned()
                            })
                            .collect();
                    }
                }
                (host, domain_data)
            }
        })
        .buffer_unordered(threads)
        .collect::<HashMap<String, DomainData>>()
        .await
}
