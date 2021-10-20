use {
    clap::{value_t, App, Arg},
    futures::stream::{self, StreamExt},
    rand::{distributions::Alphanumeric, thread_rng as rng, Rng},
    std::{
        collections::HashSet,
        net::{Ipv4Addr, SocketAddr},
    },
    tokio::{
        self,
        fs::File,
        io::{self, AsyncReadExt},
    },
    trust_dns_resolver::{
        config::{
            LookupIpStrategy, NameServerConfig, NameServerConfigGroup, Protocol, ResolverConfig,
            ResolverOpts,
        },
        name_server::{GenericConnection, GenericConnectionProvider, TokioRuntime},
        AsyncResolver, TokioAsyncResolver,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Eval args
    let matches = App::new("Rusolver")
        .version(clap::crate_version!())
        .author("Eduard Tolosa <edu4rdshl@protonmail.com>")
        .about("Fast DNS resolver written in Rust.")
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .takes_value(true)
                .help("Number of threads. Default: 100"),
        )
        .arg(
            Arg::with_name("retries")
                .long("retries")
                .takes_value(true)
                .help("Number of retries after lookup failure before giving up. Defaults to 0"),
        )
        .arg(
            Arg::with_name("domain")
                .short("d")
                .long("domain")
                .takes_value(true)
                .help("Target domain. When it's specified, a wordlist can be used from stdin for bruteforcing."),
        )
        .arg(
            Arg::with_name("resolvers")
                .short("r")
                .long("resolvers")
                .takes_value(true)
                .help("File with DNS ips."),
        )
        .arg(
            Arg::with_name("timeout")
                .long("timeout")
                .takes_value(true)
                .help("Timeout in seconds. Default: 1"),
        )
        .arg(
            Arg::with_name("ip")
                .short("i")
                .long("ip")
                .takes_value(false)
                .help("Show the discovered IP addresses. Default: false"),
        )
        .arg(
            Arg::with_name("no-verify")
                .long("no-verify")
                .takes_value(false)
                .help("Disables the double verification algorithm for valid subdomains -NOT RECOMMENDED-. Default: false"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .takes_value(false)
                .help("Enables quiet mode."),
        )
        .get_matches();

    // Assign values or use defaults
    let show_ip_adress = matches.is_present("ip");
    let threads = value_t!(matches.value_of("threads"), usize).unwrap_or_else(|_| 100);
    let timeout = value_t!(matches.value_of("timeout"), u64).unwrap_or_else(|_| 1);
    let retries = value_t!(matches.value_of("retries"), usize).unwrap_or_else(|_| 0);
    let quiet_flag = matches.is_present("quiet");
    let custom_resolvers = matches.is_present("resolvers");
    let disable_double_check = matches.is_present("no-verify") && custom_resolvers;

    // Resolver opts
    let options = ResolverOpts {
        timeout: std::time::Duration::from_secs(timeout),
        attempts: retries,
        ip_strategy: LookupIpStrategy::Ipv4Only,
        num_concurrent_reqs: 1,
        ..Default::default()
    };

    let built_in_nameservers: HashSet<String> = vec![
        // Cloudflare
        "1.1.1.1:53",
        "1.0.0.1:53",
        // Google
        "8.8.8.8:53",
        "8.8.4.4:53",
        // Quad9
        "9.9.9.9:53",
        "149.112.112.112:53",
        // OpenDNS
        "208.67.222.222:53",
        "208.67.220.220:53",
        // Verisign
        "64.6.64.6:53",
        "64.6.65.6:53",
        // UncensoredDNS
        "91.239.100.100:53",
        "89.233.43.71:53",
        // dns.watch
        "84.200.69.80:53",
        "84.200.70.40:53",
    ]
    .iter()
    .map(|x| x.to_string())
    .collect();

    // Create resolvers
    let mut nameserver_ips;

    if custom_resolvers {
        nameserver_ips =
            return_file_lines(value_t!(matches.value_of("resolvers"), String).unwrap()).await;
        nameserver_ips.retain(|ip| !ip.is_empty());
    } else {
        nameserver_ips = built_in_nameservers.clone();
    }
    let resolvers = return_tokio_asyncresolver(nameserver_ips, options);
    let trustable_resolver = return_tokio_asyncresolver(built_in_nameservers, options);
    let mut wildcard_ips = HashSet::new();

    // Read stdin
    let mut buffer = String::new();
    let mut stdin = io::stdin();
    stdin.read_to_string(&mut buffer).await?;

    let hosts: Vec<String> = if matches.is_present("domain") {
        let domain = value_t!(matches, "domain", String).unwrap();
        wildcard_ips = detect_wildcards(&domain, &trustable_resolver, quiet_flag).await;
        buffer
            .lines()
            .map(|word| format!("{}.{}", word, domain))
            .collect()
    } else {
        buffer.lines().map(str::to_owned).collect()
    };

    stream::iter(hosts)
        .map(|host| {
            let resolver_fut = resolvers.ipv4_lookup(host.clone() + ".");
            let trustable_resolver_fut = trustable_resolver.ipv4_lookup(host.clone() + ".");
            let wildcard_ips = wildcard_ips.clone();

            async move {
                let mut ips = HashSet::new();
                if let Ok(ip) = resolver_fut.await {
                    if disable_double_check {
                        ips = ip
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect::<HashSet<String>>();
                    } else if let Ok(ip) = trustable_resolver_fut.await {
                        ips = ip
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect::<HashSet<String>>();
                    }
                }
                if show_ip_adress && !ips.iter().all(|ip| wildcard_ips.contains(ip)) {
                    println!("{};{:?}", host, ips)
                } else if !ips.iter().all(|ip| wildcard_ips.contains(ip)) {
                    println!("{}", host)
                }
            }
        })
        .buffer_unordered(threads)
        .collect::<Vec<()>>()
        .await;

    Ok(())
}

// In the future I may need to implement error propagation, but for now it's fine
// to deal with matches
async fn return_file_lines(file: String) -> HashSet<String> {
    let mut f = match File::open(&file).await {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error opening resolvers file. Error: {}", e);
            std::process::exit(1)
        }
    };
    let mut buffer = String::new();

    match f.read_to_string(&mut buffer).await {
        Ok(a) => a,
        _ => unreachable!("Error reading to string."),
    };
    buffer.lines().map(|f| format!("{}:53", f)).collect()
}

fn return_tokio_asyncresolver(
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

async fn detect_wildcards(
    target: &str,
    resolvers: &AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>,
    quiet_flag: bool,
) -> HashSet<String> {
    if !quiet_flag {
        println!("Running wildcards detection for {}...\n", target)
    }
    let mut generated_wilcards: HashSet<String> = HashSet::new();
    for _ in 1..20 {
        generated_wilcards.insert(format!(
            "{}.{}.",
            rng()
                .sample_iter(Alphanumeric)
                .take(15)
                .map(char::from)
                .collect::<String>(),
            target
        ));
    }

    generated_wilcards = stream::iter(generated_wilcards.clone().into_iter().map(
        |host| async move {
            if let Ok(ips) = resolvers.ipv4_lookup(host.clone()).await {
                ips.into_iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
            } else {
                Vec::new()
            }
        },
    ))
    .buffer_unordered(10)
    .map(stream::iter)
    .flatten()
    .collect()
    .await;

    generated_wilcards.retain(|ip| ip.parse::<Ipv4Addr>().is_ok());

    if !generated_wilcards.is_empty() && !quiet_flag {
        println!(
            "Wilcards detected for {} and wildcard's IP saved for furter work.",
            target
        );
        println!("Wilcard IPs: {:?}\n", generated_wilcards)
    } else if !quiet_flag {
        println!("No wilcards detected for {}, nice!\n", target)
    }
    generated_wilcards
}
