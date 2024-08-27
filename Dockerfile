FROM rust:1.80-slim-bullseye

WORKDIR /usr/src/homebrdige-controller

ENV RUST_LOG=debug

COPY config.json config.json
COPY log4rs.yaml log4rs.yaml

RUN apt-get update && apt-get upgrade && apt-get -y install pkg-config openssl libssl-dev

RUN mkdir -p logs
RUN cargo install --git "https://github.com/jhrcook/homebridge-controller" --branch main

CMD ["homebridge-controller", "config.json"]
