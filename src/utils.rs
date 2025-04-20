use {
    futures::stream::{self, StreamExt},
    rand::{distributions::Alphanumeric, thread_rng as rng, Rng},
    std::{collections::HashSet, net::Ipv4Addr},
    tokio::{self, fs::File, io::AsyncReadExt},
    trust_dns_resolver::{
        name_server::{GenericConnection, GenericConnectionProvider, TokioRuntime},
        AsyncResolver,
    },
};

// In the future I may need to implement error propagation, but for now it's fine
// to deal with matches
pub async fn return_file_lines(file: String) -> HashSet<String> {
    let mut f = match File::open(&file).await {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error opening resolvers file. Error: {e}");
            std::process::exit(1)
        }
    };
    let mut buffer = String::new();

    match f.read_to_string(&mut buffer).await {
        Ok(a) => a,
        _ => unreachable!("Error reading to string."),
    };
    buffer.lines().map(|f| format!("{f}:53")).collect()
}

pub async fn detect_wildcards(
    target: &str,
    resolvers: &AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>,
    quiet_flag: bool,
) -> HashSet<String> {
    if !quiet_flag {
        println!("Running wildcards detection for {target}...\n");
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
        println!("Wilcards detected for {target} and wildcard's IP saved for furter work.");
        println!("Wilcard IPs: {generated_wilcards:?}\n");
    } else if !quiet_flag {
        println!("No wilcards detected for {target}, nice!\n");
    }
    generated_wilcards
}
