use {
    clap::{value_t, App, Arg},
    rusolver::{dnslib, structs, utils},
    std::collections::HashSet,
    tokio::{
        self,
        io::{self, AsyncReadExt},
    },
    trust_dns_resolver::config::{LookupIpStrategy, ResolverOpts},
};

// WIP: add support for AAAA, TXT, SRV, NAPTR, PTR, CNAME, DNAME, MX, NS, SOA, LOC, SVCB, HTTPS, SPF, CAA and AVC resource records.
// This could use a new command line option such as -t, e.g. echo www.example.com | rusolver -i -t AAAA. It might also make sense
// to change -i/--ip to -d/--data with the text Display the record data.

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
                .help("Timeout in seconds. Default: 3"),
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
    let show_ip_address = matches.is_present("ip");
    let threads = value_t!(matches.value_of("threads"), usize).unwrap_or_else(|_| 100);
    let timeout = value_t!(matches.value_of("timeout"), u64).unwrap_or_else(|_| 3);
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
        shuffle_dns_servers: true,
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
    .map(ToString::to_string)
    .collect();

    // Create resolvers
    let mut nameserver_ips;

    if custom_resolvers {
        nameserver_ips =
            utils::return_file_lines(value_t!(matches.value_of("resolvers"), String).unwrap())
                .await;
        nameserver_ips.retain(|ip| !ip.is_empty());
    } else {
        nameserver_ips = built_in_nameservers.clone();
    }
    let resolvers = dnslib::return_tokio_asyncresolver(nameserver_ips, options);
    let trustable_resolver = dnslib::return_tokio_asyncresolver(built_in_nameservers, options);
    let mut wildcard_ips = HashSet::new();

    // Read stdin
    let mut buffer = String::new();
    let mut stdin = io::stdin();
    stdin.read_to_string(&mut buffer).await?;

    let hosts: HashSet<String> = if matches.is_present("domain") {
        let domain = value_t!(matches, "domain", String).unwrap();
        wildcard_ips = utils::detect_wildcards(&domain, &trustable_resolver, quiet_flag).await;
        buffer
            .lines()
            .map(|word| format!("{word}.{domain}"))
            .collect()
    } else {
        buffer.lines().map(str::to_owned).collect()
    };

    let options = structs::LibOptions {
        hosts,
        resolvers,
        trustable_resolver,
        wildcard_ips,
        disable_double_check,
        threads,
        show_ip_address,
        quiet_flag,
    };

    dnslib::return_hosts_data(&options).await;

    Ok(())
}
