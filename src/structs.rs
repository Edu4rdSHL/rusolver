use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct DomainData {
    pub ipv4_addresses: HashSet<String>,
    pub ipv6_addresses: HashSet<String>,
    pub cname: String,
    pub is_wildcard: bool,
}

impl Default for DomainData {
    fn default() -> Self {
        DomainData {
            ipv4_addresses: HashSet::new(),
            ipv6_addresses: HashSet::new(),
            cname: String::from(""),
            is_wildcard: false,
        }
    }
}
