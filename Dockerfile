FROM rust:slim-buster

WORKDIR /build

RUN rustup default nightly
RUN rustup target add x86_64-unknown-linux-musl

RUN apt-get update -y
RUN apt-get install -y musl-tools

COPY . /build/
RUN cargo build --release --target=x86_64-unknown-linux-musl


FROM alpine

COPY --from=0 /build/target/x86_64-unknown-linux-musl/release/kaisantantoudaijin /usr/bin/kaisantantoudaijin

CMD ["/usr/bin/kaisantantoudaijin"]
