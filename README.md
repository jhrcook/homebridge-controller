# Personal Homebridge Controller

[![Rust](https://github.com/jhrcook/homebridge-controller/actions/workflows/rust.yml/badge.svg)](https://github.com/jhrcook/homebridge-controller/actions/workflows/rust.yml)

A program to execute custom Homebridge controlling programs.

## Usage

### Locally

For development:

```bash
. ./.secrets
cargo run -- config.yaml
```

### Deploy on Raspberry Pi

Download the ['compose.yaml'](./compose.yaml) and ['Dockerfile'](./Dockerfile) and run the container in the background:

```bash
# Docker files:
wget https://github.com/jhrcook/homebridge-controller/raw/main/Dockerfile
wget https://github.com/jhrcook/homebridge-controller/raw/main/compose.yaml

#> Set the secrets in the "compose.yaml" file.

# Configuration files:
wget https://github.com/jhrcook/homebridge-controller/raw/main/config.json
wget https://github.com/jhrcook/homebridge-controller/raw/main/log4rs.yaml

# Build docker
docker compose up -d
```

### Changing the configuration

You can change the configuration and copy the updated file into the running Docker container:

```bash
docker cp ./config.json <CONTAINER_NAME>:/usr/src/homebrdige-controller/config.json
```
You can find the container name using the following command (last column):

```bash
docker ps
#> CONTAINER ID   IMAGE                       COMMAND                  CREATED              STATUS         PORTS     NAMES
#> 299403e97023   homebridge-controller-app   "homebridge-controll…"   About a minute ago   Up 2 seconds             homebridge-controller-app-1
```

## Programs

Global configuration:

- `timezome`: number of hours after GMT
- `ip_addess`: Homebridge IP address

### Morning Light

Turn the light on gradually in the morning.

Configuration:

- `start`: time to start the sequence
- `duration`: duration of fading-in brightness process
- `final_brightness`: maximum brightness
- `start_hue`: starting color hue
- `final_hue`: final color hue
- `active`: whether or not this process is active

### Turning off morning light

Turn the light off later in the morning.

Notes

- Make sure to only perform this once per day.
- Can instead of a specific time set to go off a certain number of minutes after sunrise.

Configuration:

- `off_time`: time to turn the lights off in the morning
- `duration`: duration of the dimming process
- `active`: whether or not this process is active

### Turning on light in the evening

Turn the light on in the evening.

Notes

- make sure to stop the process if the light is turned off during execution

Configuration

- `hours_before_sunset_start`: number of hours before official sunset to start the sequence
- `start_brightness`: starting brightness
- `max_brightness`: maximum brightness
- `final_brightness`: final brightness
- `hours_after_sunset_end`: number of hours after sunset to finish
- `active`: whether or not this process is active
