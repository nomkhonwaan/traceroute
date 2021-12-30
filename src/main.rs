use std::io::Error;
use std::process::Command;

use chrono::{DateTime, Local};
use clap::{App, Arg};
use influxdb::{Client, InfluxDbWriteable};
use regex::Regex;

#[tokio::main]
async fn main() {
    let matches = App::new("traceroute")
        .version("0.1.0")
        .arg(
            Arg::new("destination")
                .long("destination")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("destination-port")
                .long("destination-port")
                .takes_value(true)
                .default_value("443"),
        )
        .arg(
            Arg::new("influxdb-uri")
                .long("influxdb-uri")
                .takes_value(true)
                .default_value("http://localhost:8086"),
        )
        .get_matches();

    let now = Local::now();
    let destination = matches.value_of("destination").unwrap();
    let destination_port = matches.value_of("destination-port").unwrap();
    let output = traceroute(destination, destination_port).expect("Failed to trace");
    let hops = parse(output.trim().to_string());
    let client = connect_influxdb(matches.value_of("influxdb-uri").unwrap()).unwrap();

    for hop in hops {
        for probe in hop.probes {
            let point = Point {
                hop: hop.id,
                name: probe.name,
                ip: probe.ip,
                rtt: probe.rtt,
                time: now.clone(),
            };
            client.query(point.into_query("point"))
                .await
                .expect("Failed to query");
        }
    }
}

fn connect_influxdb(uri: &str) -> Result<Client, Error> {
    let client = Client::new(uri, "traceroute");
    Ok(client)
}

fn traceroute(destination: &str, destination_port: &str) -> Result<String, Error> {
    Command::new("tcptraceroute")
        .arg(destination)
        .arg(destination_port)
        .output()
        .map(|output| String::from_utf8(output.stdout).expect("Invalid UTF-8"))
}

fn parse(s: String) -> Vec<Hop> {
    let mut hops: Vec<Hop> = Vec::new();
    if s.is_empty() {
        return hops;
    }
    for line in s.split("\n") {
        let hop: Option<Hop> = parse_hop(line);
        if let Some(hop) = hop {
            hops.push(hop);
        }
    }
    hops
}

fn parse_hop(s: &str) -> Option<Hop> {
    let mut hop = Hop::new();
    let mut parts = s.split_whitespace().collect::<Vec<&str>>();
    hop.id = parts.remove(0).parse().unwrap();
    hop.probes = parse_probes(parts);
    Some(hop)
}

fn parse_probes(mut parts: Vec<&str>) -> Vec<Probe> {
    let mut probes: Vec<Probe> = Vec::new();
    let re = Regex::new(r"\(.+\)").unwrap();

    while parts.len() > 0 {
        let tok1 = parts.remove(0);
        if tok1 == "*" {
            continue;
        }

        let tok2 = parts.remove(0);
        let mut probe = Probe::new();

        if re.is_match(tok2) {  // ve474.cgn05.cwdc.myaisfibre.com (49.228.4.38)
            probe.name = tok1.to_string();
            probe.ip = tok2[1..tok2.len() - 1].to_string();
            probe.rtt = parts.remove(0).parse()
                .or_else(|_| parts.remove(0).parse()).unwrap();
            parts.remove(0); // Drop "ms"
        } else if tok1 == "[open]" {  // kul09s16-in-f3.1e100.net (216.58.200.3) [open]  31.237 ms
            let prev = probes.last().unwrap();
            probe.name = prev.name.clone();
            probe.ip = prev.ip.clone();
            probe.rtt = tok2.parse().unwrap();
            parts.remove(0); // Drop "ms"
        } else if tok2 == "[open]" { // 104.18.25.25 [open]  5.890 ms
            probe.name = tok1.to_string();
            probe.ip = tok1.to_string();
            probe.rtt = parts.remove(0).parse().unwrap();
            parts.remove(0); // Drop "ms"
        } else if tok2 == "ms" { // 5.890 ms
            let prev = probes.last().unwrap();
            probe.name = prev.name.clone();
            probe.ip = prev.ip.clone();
            probe.rtt = tok1.parse().unwrap();
        } else { // 49-228-0-0.24.cwdc.myaisfibre.com (49.228.0.252)  5.332 ms
            probe.name = tok1.to_string();
            probe.ip = tok1.to_string();
            probe.rtt = tok2.parse().unwrap();
            parts.remove(0); // Drop "ms"
        }

        probes.push(probe);
    }

    probes
}

#[derive(Debug)]
struct Hop {
    id: u8,
    probes: Vec<Probe>,
}

impl Hop {
    fn new() -> Self {
        Hop {
            id: 0,
            probes: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct Probe {
    name: String,
    ip: String,
    rtt: f64,
}

impl Probe {
    fn new() -> Self {
        Probe {
            ip: String::new(),
            name: String::new(),
            rtt: 0.0,
        }
    }
}

#[derive(InfluxDbWriteable)]
struct Point {
    #[influxdb(tag)] hop: u8,
    name: String,
    ip: String,
    rtt: f64,
    time: DateTime<Local>,
}