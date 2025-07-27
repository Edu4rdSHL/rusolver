use {hickory_resolver::TokioResolver, std::collections::HashSet};

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
    pub resolvers: TokioResolver,
    pub trustable_resolvers: TokioResolver,
    pub wildcard_ips: HashSet<String>,
    pub enable_double_check: bool,
    pub threads: usize,
    pub show_ip_address: bool,
    pub quiet_flag: bool,
}
