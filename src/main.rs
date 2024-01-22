use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use std::error::Error;
use clap::Parser;
use sysinfo::System;
use std::process::Command;
use std::str;
use reqwest::blocking::Client;

#[derive(Parser, Debug)]
#[command(author,version,about, long_about=None)]
struct Args {
    /// Interval for querying in seconds
    #[arg(short, long)]
    interval: u64,

    /// Optionally exclude GPU
    #[arg(long)]
    exclude_gpu: bool,

    /// InfluxDB URL
    #[arg(long)]
    influxdb_url: String,

    /// InfluxDB Token
    #[arg(long)]
    influxdb_token: String,

    /// InfluxDB Organization
    #[arg(long)]
    influxdb_org: String,

    /// InfluxDB Bucket
    #[arg(long)]
    influxdb_bucket: String,
    
}

struct NetworkTraffic {
    bytes_received: u64,
    bytes_sent: u64,
    timestamp: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Command line arguments and parsing
    let args: Args = Args::parse();
    let interval: u64 = args.interval;
    let exclude_gpu: bool = args.exclude_gpu;
    
    // Initialize system info
    let mut sys: System = System::new_all();
    let mut net_traffic: NetworkTraffic = NetworkTraffic::new();

    let client: Client = Client::new();
    
    loop {
        // Get basic parameters
        let now: SystemTime = SystemTime::now();
        let timestamp: u128 = now.duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos(); 

        let cpu_usage: f32 = get_cpu_usage(&mut sys);
        let ram_usage: f32 = get_ram_usage(&mut sys);
        let heaviest_process_name: String = get_heaviest_process(&mut sys);

        // Get network traffic data
        let (download_rate, upload_rate) = match net_traffic.update() {
            Ok((download_rate, upload_rate)) => (download_rate, upload_rate),
            Err(e) => {
                eprintln!("Failed to get network traffic: {}", e);
                (-1.0, -1.0) // Use -1 as values for download/upload if net_traffic.update() fails
            }
        };
        
        // Get GPU usage data 
        let (gpu_usage, gpu_temp, gpu_power) = get_gpu_info(exclude_gpu);
        

        // Prepare data for sending to InfluxDB
        let data: String = format!(
            "system_metrics,host=localhost cpu_usage={},ram_usage={},heaviest_process=\"{}\",gpu_usage={},gpu_temp={},gpu_power={},download_rate={},upload_rate={} {}",
            cpu_usage,
            ram_usage,
            heaviest_process_name,
            gpu_usage,
            gpu_temp,
            gpu_power,
            download_rate,
            upload_rate,
            timestamp
        );
        
        // Prepare Client and post response
        
        let response: Result<reqwest::blocking::Response, reqwest::Error> = client.post(&format!("{}/api/v2/write?org={}&bucket={}", args.influxdb_url, args.influxdb_org, args.influxdb_bucket))
            .header("Authorization", format!("Token {}", args.influxdb_token))
            .body(data)
            .send();
        
        match response {
            Ok(resp) => println!("Data sent to InfluxDB, status: {}", resp.status()),
            Err(e) => eprintln!("Failed to send data to InfluxDB: {}", e),
        }
        
        thread::sleep(Duration::from_secs(interval));
    }
}

impl NetworkTraffic {
    /// Creates a new NetworkTraffic instance with all fields set to zero.
    fn new() -> Self {
        Self {
            bytes_received: 0,
            bytes_sent: 0,
            timestamp: 0,
        }
    }
    /// Updates the NetworkTraffic instance with the current network traffic data.
    /// Returns the download and upload rates in bytes/sec.
    fn update(&mut self) -> Result<(f64, f64), Box<dyn std::error::Error>> {
        let output: std::process::Output = Command::new("cmd")
            .args(&["/C", "netstat", "-e"])
            .output()?;

        let output_str: &str = str::from_utf8(&output.stdout)?;

        let lines: Vec<&str> = output_str.lines().collect();
        if lines.len() < 5 {
            return Err("Unexpected output from netstat -e".into());
        }

        let parts: Vec<&str> = lines[4].split_whitespace().collect();
        if parts.len() < 3 {
            return Err("Unexpected output from netstat -e".into());
        }

        let bytes_received: u64 = parts[1].parse()?;
        let bytes_sent: u64 = parts[2].parse()?;

        let now: u64 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let download_rate: f64 = if self.timestamp > 0 {
            (bytes_received - self.bytes_received) as f64 / (now - self.timestamp) as f64
        } else {
            0.0
        };

        let upload_rate: f64 = if self.timestamp > 0 {
            (bytes_sent - self.bytes_sent) as f64 / (now - self.timestamp) as f64
        } else {
            0.0
        };

        self.bytes_received = bytes_received;
        self.bytes_sent = bytes_sent;
        self.timestamp = now;

        Ok((download_rate, upload_rate))
    }
}



fn get_cpu_usage(sys: &mut System) -> f32 {
    sys.refresh_cpu();
    sys.global_cpu_info().cpu_usage()
}

fn get_ram_usage(sys: &mut System) -> f32 {
    sys.refresh_memory();
    sys.used_memory() as f32 / 1024.0 / 1024.0 / 1024.0
}

fn get_heaviest_process(sys: &mut System) -> String {
    sys.refresh_all();

    let mut max_cpu_usage: f32 = 0.0;
    let mut heaviest_process_name: String = String::new();

    for (_pid, proc) in sys.processes() {
        let cpu_usage: f32 = proc.cpu_usage();
        if cpu_usage > max_cpu_usage {
            max_cpu_usage = cpu_usage;
            heaviest_process_name = proc.name().to_string();
        }
    }

    heaviest_process_name
}

fn get_gpu_info(exclude_gpu: bool) -> (f32, f32, f32) {
    if exclude_gpu {
        return (-1.0, -1.0, -1.0);
    }

    let output: Result<std::process::Output, std::io::Error> = Command::new("nvidia-smi")
        .arg("--query-gpu=utilization.gpu,temperature.gpu,power.draw")
        .arg("--format=csv,noheader,nounits")
        .output();

    match output {
        Ok(output) => {
            let output_str: &str = str::from_utf8(&output.stdout).unwrap();
            let splits: Vec<&str> = output_str.split(", ").collect();
            (
                splits[0].trim().parse::<f32>().unwrap_or(-1.0),
                splits[1].trim().parse::<f32>().unwrap_or(-1.0),
                splits[2].trim().parse::<f32>().unwrap_or(-1.0),
            )
        }
        // If there is an error, just return -1 as values
        Err(_) => (-1.0, -1.0, -1.0),
    }
}
