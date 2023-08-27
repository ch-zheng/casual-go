FROM rust:latest as builder
WORKDIR /usr/src/casual-go
COPY . .
RUN cargo install --path .

FROM debian:stable-slim
RUN apt-get update && apt-get install -y gnugo && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/casual-go /usr/local/bin/casual-go

RUN useradd -r appuser
USER appuser
EXPOSE 50000
CMD ["casual-go", "50000", "/usr/games/gnugo"]
