use clap::{arg, Parser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        default_value_t = 100,
        help = "Number of threads. Default: 100"
    )]
    pub threads: usize,

    #[arg(
        long,
        default_value_t = 0,
        help = "Number of retries after lookup failure before giving up. Defaults to 0"
    )]
    pub retries: usize,

    #[arg(
        short,
        long,
        help = "Target domain. When it's specified, a wordlist can be used from stdin for bruteforcing."
    )]
    pub domain: Option<String>,

    #[arg(short, long, help = "File with DNS ips.")]
    pub resolvers: Option<String>,

    #[arg(long, default_value_t = 3, help = "Timeout in seconds. Default: 3")]
    pub timeout: u64,

    #[arg(short, long, help = "Display the record data.")]
    pub ip: bool,

    #[arg(
        short,
        long,
        help = "Enable the double verification algorithm for subdomains. Default: false"
    )]
    pub enable_double_check: bool,

    #[arg(short, long, help = "Quiet mode, no output except errors.")]
    pub quiet_flag: bool,
}
