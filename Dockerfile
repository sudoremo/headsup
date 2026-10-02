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

RUN apk add --no-cache ca-certificates tzdata libgcc libstdc++ ripgrep

# Claude Code for the claude backend, from Anthropic's signed apk repository.
# The key is checked against the published checksum, so a substituted key fails
# the build rather than the digest run.
RUN wget -q -O /etc/apk/keys/claude-code.rsa.pub \
      https://downloads.claude.ai/keys/claude-code.rsa.pub && \
    echo "395759c1f7449ef4cdef305a42e820f3c766d6090d142634ebdb049f113168b6  /etc/apk/keys/claude-code.rsa.pub" \
      | sha256sum -c - && \
    echo "https://downloads.claude.ai/claude-code/apk/stable" >> /etc/apk/repositories && \
    apk add --no-cache claude-code

# The ripgrep Claude Code bundles is linked against glibc, so it has to use the
# one from apk instead
ENV USE_BUILTIN_RIPGREP=0

COPY --from=builder /app/target/*/release/headsup /usr/local/bin/headsup

# Create non-root user
RUN adduser -D -u 1000 headsup
USER headsup

WORKDIR /app
ENTRYPOINT ["/usr/local/bin/headsup"]
CMD ["check"]
