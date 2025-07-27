use {
    crate::structs::{DomainData, LibOptions},
    futures::stream::{self, StreamExt},
    hickory_resolver::{
        config::{LookupIpStrategy, ResolverOpts, ServerOrderingStrategy},
        TokioResolver,
    },
    rand::{distr::Alphanumeric, rng, Rng},
    std::{collections::HashSet, net::Ipv4Addr},
    tokio::{fs::File, io::AsyncReadExt},
};

pub async fn return_file_lines(file: &str) -> HashSet<String> {
    let mut f = match File::open(file).await {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error opening resolvers file. Error: {e}");
            std::process::exit(1)
        }
    };
    let mut buffer = String::new();

    (f.read_to_string(&mut buffer).await)
        .unwrap_or_else(|_| unreachable!("Error reading to string."));

    let estimated_lines = buffer.matches('\n').count() + 1;
    let mut result = HashSet::with_capacity(estimated_lines);

    for line in buffer.lines() {
        if !line.is_empty() {
            result.insert(format!("{line}:53"));
        }
    }
    result
}

pub async fn detect_wildcards(
    target: &str,
    resolvers: &TokioResolver,
    quiet_flag: bool,
) -> HashSet<String> {
    if !quiet_flag {
        println!("Running wildcards detection for {target}...\n");
    }

    let mut generated_wildcards = HashSet::with_capacity(19);

    // Generate random subdomains for wildcard detection
    for _ in 1..20 {
        let random_subdomain: String = rng()
            .sample_iter(Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();
        generated_wildcards.insert(format!("{random_subdomain}.{target}."));
    }

    let wildcard_ips: HashSet<String> = stream::iter(generated_wildcards.into_iter())
        .map(|host| async move {
            resolvers.ipv4_lookup(host).await.map_or_else(
                |_| Vec::new(),
                |ips| {
                    ips.into_iter()
                        .filter_map(|ip| {
                            let ip_str = ip.to_string();
                            if ip_str.parse::<Ipv4Addr>().is_ok() {
                                Some(ip_str)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<String>>()
                },
            )
        })
        .buffer_unordered(10)
        .map(stream::iter)
        .flatten()
        .collect()
        .await;

    if !wildcard_ips.is_empty() && !quiet_flag {
        println!("Wildcards detected for {target} and wildcard's IP saved for further work.");
        println!("Wildcard IPs: {wildcard_ips:?}\n");
    } else if !quiet_flag {
        println!("No wildcards detected for {target}, nice!\n");
    }
    wildcard_ips
}

pub fn print_domain_data(host: &str, domain_data: &DomainData, options: &LibOptions) {
    if options.show_ip_address && !domain_data.is_wildcard {
        println!("{}: {:?}", host, domain_data.ipv4_addresses);
    } else {
        println!("{}", host);
    }
}

pub fn return_resolver_opts(timeout: u64, retries: usize) -> ResolverOpts {
    let mut options = ResolverOpts::default();
    options.timeout = std::time::Duration::from_secs(timeout);
    options.attempts = retries;
    options.ip_strategy = LookupIpStrategy::Ipv4Only;
    options.num_concurrent_reqs = 1;
    options.server_ordering_strategy = ServerOrderingStrategy::RoundRobin;
    options
}
