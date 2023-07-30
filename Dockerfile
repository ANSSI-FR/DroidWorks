FROM rust:latest AS builder
WORKDIR /usr/src/droidworks
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/droidworks /usr/local/bin/
COPY --from=builder /usr/local/cargo/bin/dw-* /usr/local/bin/
COPY --from=builder /usr/local/cargo/bin/zipalign /usr/local/bin/
