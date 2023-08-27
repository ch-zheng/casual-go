FROM rust:1.67 as builder
WORKDIR /usr/src/casual-go
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y gnugo && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/casual-go /usr/local/bin/casual-go
EXPOSE 80
CMD ["casual-go"]
