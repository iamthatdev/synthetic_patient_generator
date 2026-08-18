# Build stage
FROM rust:1.75-alpine AS builder

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source
COPY src ./src
COPY config ./config

# Build release binary
RUN cargo build --release

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy binary
COPY --from=builder /build/target/release/synthetic-patient-gen /usr/local/bin/synthetic-patient-gen

# Copy config
COPY --from=builder /build/config ./config

# Create output directory
VOLUME ["/output"]

WORKDIR /output

ENTRYPOINT ["synthetic-patient-gen"]
CMD ["generate", "--help"]
