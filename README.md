# system-monitoring
A system resource monitoring project utilizing Rust and InfluxDB. Uses `influxdb==2.7` and was compiled with `rustc=1.75`. A Dockerfile is included to containerize and automate the setup of the InfluxDB instance, if a local setup is not already available. 

# Usage

## Docker setup
If you want to use the included dockerfile, first create an .env file with the following structure:

```
DOCKER_INFLUXDB_INIT_MODE=setup
DOCKER_INFLUXDB_INIT_USERNAME=your_username
DOCKER_INFLUXDB_INIT_PASSWORD=your_password
DOCKER_INFLUXDB_INIT_ORG=your_org
DOCKER_INFLUXDB_INIT_BUCKET=your_bucket
DOCKER_INFLUXDB_INIT_ADMIN_TOKEN=your_token
```

Then run:  `docker run --env-file /path/to/env/file -p 8086:8086 -v /path/to/mount/locally:/var/lib/influxdb2 automated-influxdb`. This will initialize InfluxDB with the parameters provided on the env file.

## Application
`cargo run --release -- --interval INTERVAL_IN_SECONDS --influxdb-url http://localhost:8086 --influxdb-token your_token --influxdb-org your_org --influxdb-bucket your_bucket`. If `--exclude-gpu` is added, the application will return -1 for the GPU parameters. The application will also return -1 for the network parameters if `netstat` fails. 

# Footprint

The application is very lightweight when compiled with the `--release` flag, using `<10 MB` of memory and barely utilizing the CPU. It sends data to InfluxDB via HTTP requests locally.  

# Example dashboard (Influx UI)
![Inlux UI Dashboard](https://github.com/ikeratzakis/system-monitoring/blob/main/images/dashboard.png?raw=true)
