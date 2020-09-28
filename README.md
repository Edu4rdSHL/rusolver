# Rusolver
Fast DNS resolver written in Rust. Works on Linux, Windows, macOS, Android, Aarch64, ARM and possibly in your oven.

# Goal
Offer the community an efficient host resolution tool.

# Performance & speed
Rusolver is **very** resource friendly, you can use up to 1000 threads in an single core machine and this will work without any problem, the bottleneck for this tool is your network speed. By default, Rusolver is able to perform resolution for ~1532 hosts per second in good network conditions (tested in an AWS machine).

## Demo
[![asciicast](https://asciinema.org/a/362323.svg)](https://asciinema.org/a/362323)

# Installation

## Using precompiled binaries.

Download the asset from the [releases page](https://github.com/Edu4rdSHL/rusolver/releases/latest) according to your platform.

## Using the source code.

1. You need to have the lastest stable [Rust](https://www.rust-lang.org/) version insalled in your system.
2. Clone the repo or download the source code, then run `cargo build --release`.
3. Execute the tool from `./target/release/rusolver` or add it to your system PATH to use from anywhere.

Optionally you can just use `cargo install rusolver`

# Usage
* By default we only show the resolved hosts
```
cat hosts.txt | rusolver
```
* If you want to see the discovered IP addresses:
```
cat hosts.txt | rusolver -i
```
You can tune the `--timeout` and `-t/--threads` options according to your needs. See `rusolver --help`
