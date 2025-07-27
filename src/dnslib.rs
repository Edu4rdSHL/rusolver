use {
    crate::{
        structs::{DomainData, LibOptions},
        utils::print_domain_data,
    },
    futures::stream::{self, StreamExt},
    hickory_resolver::{
        config::{NameServerConfig, NameServerConfigGroup, ResolverConfig, ResolverOpts},
        name_server::TokioConnectionProvider,
        proto::{rr::RecordType, xfer::Protocol},
        TokioResolver,
    },
    std::{
        collections::{HashMap, HashSet},
        net::SocketAddr,
    },
};

#[must_use]
pub fn return_tokio_asyncresolver<S: ::std::hash::BuildHasher>(
    nameserver_ips: &HashSet<String, S>,
    options: ResolverOpts,
) -> TokioResolver {
    let mut name_servers = NameServerConfigGroup::with_capacity(nameserver_ips.len());

    name_servers.extend(nameserver_ips.iter().map(|server| {
        let socket_addr = SocketAddr::V4(server.parse().unwrap_or_else(|e| {
            panic!(
                "Error parsing the server {}, only IPv4 are allowed. Error: {}",
                server, e
            )
        }));

        NameServerConfig {
            socket_addr,
            protocol: Protocol::Udp,
            tls_dns_name: None,
            http_endpoint: None,
            trust_negative_responses: false,
            bind_addr: None,
        }
    }));

    TokioResolver::builder_with_config(
        ResolverConfig::from_parts(None, vec![], name_servers),
        TokioConnectionProvider::default(),
    )
    .with_options(options)
    .build()
}

pub async fn return_hosts_data(options: &LibOptions) -> HashMap<String, DomainData> {
    let threads = if options.hosts.len() < options.threads {
        options.hosts.len()
    } else {
        options.threads
    };

    stream::iter(options.hosts.clone().into_iter().map(|host| {
        let lookup_host = format!("{}.", host.trim_end_matches('.'));
        let wildcard_ips = options.wildcard_ips.clone();
        let mut domain_data = DomainData::default();

        async move {
            let ip_lookup = if options.enable_double_check {
                match (
                    options.resolvers.ipv4_lookup(lookup_host.clone()).await,
                    options.trustable_resolvers.ipv4_lookup(lookup_host).await,
                ) {
                    (Ok(_), Ok(ip)) => Some(ip),
                    _ => None,
                }
            } else {
                options.resolvers.ipv4_lookup(lookup_host).await.ok()
            };

            if let Some(ip_lookup) = ip_lookup {
                for ip in ip_lookup.iter() {
                    domain_data.ipv4_addresses.insert(ip.to_string());
                }
            }

            domain_data.is_wildcard = domain_data
                .ipv4_addresses
                .iter()
                .all(|ip| wildcard_ips.contains(ip));

            print_domain_data(&host, &domain_data, &options);

            (host, domain_data)
        }
    }))
    .buffer_unordered(threads)
    .collect::<HashMap<String, DomainData>>()
    .await
}

// Used internally for now
pub async fn return_cname_data<S: ::std::hash::BuildHasher>(
    hosts: HashSet<String, S>,
    resolver: TokioResolver,
    trustable_resolver: TokioResolver,
    disable_double_check: bool,
    mut threads: usize,
) -> HashMap<String, DomainData> {
    if hosts.len() < threads {
        threads = hosts.len();
    }

    stream::iter(hosts)
        .map(|host| {
            let host = host.trim_end_matches('.').to_owned();
            let fqdn = format!("{host}.");

            let resolver = resolver.clone();
            let trustable_resolver = trustable_resolver.clone();

            async move {
                let cname_lookup = if disable_double_check {
                    resolver.lookup(fqdn.clone(), RecordType::CNAME).await.ok()
                } else {
                    match (
                        resolver.lookup(fqdn.clone(), RecordType::CNAME).await,
                        trustable_resolver.lookup(fqdn, RecordType::CNAME).await,
                    ) {
                        (Ok(_), Ok(lookup)) => Some(lookup),
                        _ => None,
                    }
                };

                let mut domain_data = DomainData::default();

                if let Some(lookup) = cname_lookup {
                    for record in lookup.iter() {
                        if let Some(cname) = record.as_cname() {
                            domain_data.cname = cname.to_string();
                            break;
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
