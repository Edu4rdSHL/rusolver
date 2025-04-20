use {
    std::collections::HashSet,
    trust_dns_resolver::{
        name_server::{GenericConnection, GenericConnectionProvider, TokioRuntime},
        AsyncResolver,
    },
};

#[derive(Clone, Debug, Default)]
pub struct DomainData {
    pub ipv4_addresses: HashSet<String>,
    pub ipv6_addresses: HashSet<String>,
    pub cname: String,
    pub is_wildcard: bool,
}

#[derive(Clone, Debug)]
pub struct LibOptions {
    pub hosts: HashSet<String>,
    pub resolvers: AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>,
    pub trustable_resolver:
        AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>,
    pub wildcard_ips: HashSet<String>,
    pub disable_double_check: bool,
    pub threads: usize,
    pub show_ip_address: bool,
    pub quiet_flag: bool,
}
