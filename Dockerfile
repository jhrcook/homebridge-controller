FROM rust:1.80.1

WORKDIR /usr/src/homebrdige-controller

ENV RUST_LOG=debug

COPY . .
COPY config.json config.json
COPY secrets.json secrets.json

# RUN apk update
# RUN apk add --no-cache musl-dev
# RUN apk add --no-cache pkgconf openssl-dev

RUN cargo install --git "https://github.com/jhrcook/homebridge-controller" --branch dev

CMD ["homebridge-controller", "config.json"]
