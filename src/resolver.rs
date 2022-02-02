use {
    std::{collections::HashSet, net::SocketAddr},
    trust_dns_resolver::{
        config::{NameServerConfig, NameServerConfigGroup, Protocol, ResolverConfig, ResolverOpts},
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
