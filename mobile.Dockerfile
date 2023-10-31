FROM rust:1.72 AS dependencies

RUN apt-get update && apt-get install -y protobuf-compiler

# Copy cargo files and workspace dependencies to cache build
COPY Cargo.toml Cargo.lock ./
COPY db_store ./db_store/
COPY file_store ./file_store/
COPY metrics ./metrics/
COPY price ./price/
COPY reward_scheduler ./reward_scheduler/
COPY solana ./solana/
COPY task_manager ./task_manager/

# Copy service/binary cargo files for stub builds
COPY ingest/Cargo.toml ./ingest/Cargo.toml
COPY mobile_config/Cargo.toml ./mobile_config/Cargo.toml
COPY mobile_packet_verifier/Cargo.toml ./mobile_packet_verifier/Cargo.toml
COPY mobile_verifier/Cargo.toml ./mobile_verifier/Cargo.toml
COPY reward_index/Cargo.toml ./reward_index/Cargo.toml

# Enable sparse registry to avoid crates indexing infinite loop
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN bash -c 'mkdir {ingest,mobile_config,mobile_packet_verifier,mobile_verifier,reward_index}/src' \
 # Create a dummy project files to build deps around
 && echo "fn main() {}" > ./ingest/src/main.rs \
 && echo "fn main() {}" > ./mobile_config/src/main.rs \
 && echo "fn main() {}" > ./mobile_packet_verifier/src/main.rs \
 && echo "fn main() {}" > ./mobile_verifier/src/main.rs \
 && echo "fn main() {}" > ./reward_index/src/main.rs \
 # Remove unused workspace members to avoid compile error on missing members
 && sed -i -e '/denylist/d' \
           -e '/iot_config/d' \
           -e '/iot_packet_verifier/d' \
           -e '/iot_verifier/d' \
           -e '/poc_entropy/d' \
           -e '/mobile_config_cli/d' \
           Cargo.toml \
 && cargo build --release

# Compile the ingest service
FROM dependencies as builder-ingest
COPY ingest ./ingest/
RUN cargo build --package ingest --release

# Compile reward-index service
FROM dependencies as builder-rewarder
COPY reward_index ./reward_index/
RUN cargo build --package reward-index --release

# Compile the mobile-config service
FROM dependencies as builder-config
COPY mobile_config ./mobile_config/
RUN cargo build --package mobile-config --release

# Compile mobile-packet-verifier service
FROM builder-config as builder-packet-verifier
COPY mobile_packet_verifier ./mobile_packet_verifier/
RUN cargo build --package mobile-packet-verifier --release

# Compile mobile-verifier service
FROM builder-config as builder-verifier
COPY mobile_verifier ./mobile_verifier/
RUN cargo build --package mobile-verifier --release

# Wrap it all up with a bow
FROM debian:bullseye-slim

COPY --from=builder-ingest ./target/release/ingest /opt/ingest/bin/ingest
COPY --from=builder-config ./target/release/mobile-config /opt/mobile-config/bin/mobile-config
COPY --from=builder-packet-verifier ./target/release/mobile-packet-verifier /opt/mobile-packet-verifier/bin/mobile-packet-verifier
COPY --from=builder-verifier ./target/release/mobile-verifier /opt/mobile-verifier/bin/mobile-verifier
COPY --from=builder-rewarder ./target/release/reward-index /opt/reward-index/bin/reward-index
COPY --from=dependencies ./target/release/price /opt/price/bin/price
