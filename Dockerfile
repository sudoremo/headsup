# Build stage
FROM rust:1.84-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconf

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build with static linking for portable binary
ENV OPENSSL_STATIC=1
RUN cargo build --release --target x86_64-unknown-linux-musl || cargo build --release

# Runtime stage
FROM alpine:3.21

RUN apk add --no-cache ca-certificates tzdata

COPY --from=builder /app/target/*/release/headsup /usr/local/bin/headsup

# Create non-root user
RUN adduser -D -u 1000 headsup
USER headsup

WORKDIR /app
ENTRYPOINT ["/usr/local/bin/headsup"]
CMD ["check"]
