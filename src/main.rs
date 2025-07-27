use {
    clap::Parser,
    rusolver::{args::Args, dnslib, structs, utils},
    std::collections::HashSet,
    tokio::{
        self,
        io::{self, AsyncReadExt},
    },
};

// WIP: add support for AAAA, TXT, SRV, NAPTR, PTR, CNAME, DNAME, MX, NS, SOA, LOC, SVCB, HTTPS, SPF, CAA and AVC resource records.
// This could use a new command line option such as -t, e.g. echo www.example.com | rusolver -i -t AAAA. It might also make sense
// to change -i/--ip to -d/--data with the text Display the record data.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Eval args
    let args = Args::parse();

    // Resolver opts
    let options = utils::return_resolver_opts(args.timeout, args.retries);

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
    ]
    .iter()
    .map(ToString::to_string)
    .collect();

    // Create resolvers
    let mut nameserver_ips;

    if args.resolvers.is_some() {
        nameserver_ips = utils::return_file_lines(&args.resolvers.unwrap()).await;
        nameserver_ips.retain(|ip| !ip.is_empty());
    } else {
        nameserver_ips = built_in_nameservers.clone();
    }

    let resolvers = dnslib::return_tokio_asyncresolver(&nameserver_ips, options.clone());
    let trustable_resolvers = dnslib::return_tokio_asyncresolver(&built_in_nameservers, options);
    let mut wildcard_ips = HashSet::new();

    // Read stdin
    let mut buffer = String::new();
    let mut stdin = io::stdin();
    stdin.read_to_string(&mut buffer).await?;

    let hosts: HashSet<String> = if args.domain.is_some() {
        let domain = args.domain.unwrap();
        wildcard_ips =
            utils::detect_wildcards(&domain, &trustable_resolvers, args.quiet_flag).await;
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
        trustable_resolvers,
        wildcard_ips,
        enable_double_check: args.enable_double_check,
        threads: args.threads,
        show_ip_address: args.ip,
        quiet_flag: args.quiet_flag,
    };

    dnslib::return_hosts_data(&options).await;

    Ok(())
}
