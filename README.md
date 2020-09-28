# Rusolver
Fast DNS resolver written in Rust.

# Goal
Offer the community an efficient host resolution tool.

# Performance & speed
Rusolver is **very** resource friendly, you can use up to 1000 threads in an single core machine and this will work without any problem, the bottleneck for this tool is your network speed. By default, Rusolver is able to perform resolution for ~1226 hosts per second in good network conditions (tested in an AWS machine).

```bash
#
# hosts.txt is a list of 61309 Google subdomains. See https://gist.github.com/Edu4rdSHL/90ddc4742b816439a112a95039a95312
#
$ cat hosts.txt | rusolver
...
real	0m50.222s
user	0m17.152s
sys	0m10.064s

$ python
Python 2.7.12 (default, Jul 21 2020, 15:19:50) 
[GCC 5.4.0 20160609] on linux2
Type "help", "copyright", "credits" or "license" for more information.
>>> 61309/50
1226
>>> 

# 1226 hosts were resolved per second in average
```

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
