use {
    clap::{value_t, App, Arg},
    futures::stream::{self, StreamExt},
    rand::{distributions::Alphanumeric, prelude::SliceRandom, thread_rng as rng, Rng},
    std::{
        collections::HashSet,
        net::{IpAddr, Ipv4Addr},
    },
    tokio::{
        self,
        fs::File,
        io::{self, AsyncReadExt},
    },
    trust_dns_resolver::{
        config::{NameServerConfigGroup, ResolverConfig, ResolverOpts},
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
        ).arg(
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
    let quiet_flag = matches.is_present("quiet");

    // Resolver opts
    let options = ResolverOpts {
        timeout: std::time::Duration::from_secs(timeout),
        ..Default::default()
    };

    // Create resolvers
    let mut dns_ips = HashSet::new();

    if matches.is_present("resolvers") {
        dns_ips = return_file_lines(value_t!(matches.value_of("resolvers"), String).unwrap()).await;
    } else {
        let built_in_dns = vec![
            // Cloudflare
            "1.1.1.1",
            "1.0.0.1",
            // Google
            "8.8.8.8",
            "8.8.4.4",
            // Quad9
            "9.9.9.9",
            "149.112.112.112",
            // OpenDNS
            "208.67.222.222",
            "208.67.220.220",
            // Verisign
            "64.6.64.6",
            "64.6.65.6",
            // UncensoredDNS
            "91.239.100.100",
            "89.233.43.71",
            // dns.watch
            "84.200.69.80",
            "84.200.70.40",
        ];
        for ip in built_in_dns {
            dns_ips.insert(ip.to_string());
        }
    }
    let resolvers = return_tokio_dns(dns_ips, options).await;
    let mut wildcard_ips = HashSet::new();

    // Read stdin
    let mut buffer = String::new();
    let mut stdin = io::stdin();
    stdin.read_to_string(&mut buffer).await?;

    let hosts: Vec<String> = if matches.is_present("domain") {
        let domain = value_t!(matches, "domain", String).unwrap();
        wildcard_ips = detect_wildcards(&domain, &resolvers, quiet_flag).await;
        buffer
            .lines()
            .map(|word| format!("{}.{}", word, domain))
            .collect()
    } else {
        buffer.lines().map(str::to_owned).collect()
    };

    futures::stream::iter(hosts.into_iter().map(|host| {
        let resolver_fut = resolvers
            .choose(&mut rng())
            .expect("failed to retrieve DNS resolver")
            .ipv4_lookup(host.clone());
        let wildcard_ips = wildcard_ips.clone();
        async move {
            if let Ok(ip) = resolver_fut.await {
                let ips = ip
                    .into_iter()
                    .map(|x| x.to_string())
                    .collect::<HashSet<String>>();
                if show_ip_adress && !ips.iter().all(|ip| wildcard_ips.contains(ip)) {
                    println!("{};{:?}", host, ips)
                } else if !ips.iter().all(|ip| wildcard_ips.contains(ip)) {
                    println!("{}", host)
                }
            }
        }
    }))
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
    buffer.lines().map(str::to_owned).collect()
}

async fn return_tokio_dns(
    dns_ips: HashSet<String>,
    options: ResolverOpts,
) -> Vec<AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>> {
    let mut resolvers = Vec::new();
    for ip in dns_ips {
        resolvers.push(
            TokioAsyncResolver::tokio(
                ResolverConfig::from_parts(
                    None,
                    vec![],
                    NameServerConfigGroup::from_ips_clear(
                        &[IpAddr::V4(match ip.parse() {
                            Ok(a) => a,
                            Err(e) => {
                                eprintln!(
                                    "Error adding {} to the list of resolvers, only IPv4 addresses are allowed. Please fix the problem and try again. Error: {}",
                                    ip, e
                                );
                                std::process::exit(1)
                            }
                        })],
                        53,
                        false,
                    ),
                ),
                options,
            )
            .unwrap(),
        )
    }
    resolvers
}

async fn detect_wildcards(
    target: &str,
    resolvers: &[AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>],
    quiet_flag: bool,
) -> HashSet<String> {
    if !quiet_flag {
        println!("Running wildcards detection for {}...\n", target)
    }
    let mut generated_wilcards: HashSet<String> = HashSet::new();
    for _ in 1..10 {
        generated_wilcards.insert(format!(
            "{}.{}",
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
            if let Ok(ips) = resolvers
                .choose(&mut rng())
                .expect("failed to retrieve DNS resolver")
                .ipv4_lookup(host.clone())
                .await
            {
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
