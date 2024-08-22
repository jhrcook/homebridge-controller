FROM rust:1.80.1-alpine

WORKDIR /usr/src/homebrdige-controller

ENV RUST_LOG=debug

COPY config.json config.json
COPY secrets.json secrets.json

RUN apk update
RUN apk add --no-cache musl-dev pkgconf libressl-dev

RUN cargo install --git "https://github.com/jhrcook/homebridge-controller" --branch main

CMD ["homebridge-controller", "config.json"]
