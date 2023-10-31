FROM rust:1.72 AS dependencies

RUN apt-get update && apt-get install -y protobuf-compiler

# Copy cargo files and workspace dependencies to cache build
COPY Cargo.toml Cargo.lock ./
COPY db_store ./db_store/
COPY denylist ./denylist/
COPY file_store ./file_store/
COPY metrics ./metrics/
COPY price ./price/
COPY reward_scheduler ./reward_scheduler/
COPY solana ./solana/
COPY task_manager ./task_manager/

# Copy service/binary cargo files for stub builds
COPY ingest/Cargo.toml ./ingest/Cargo.toml
COPY iot_config/Cargo.toml ./iot_config/Cargo.toml
COPY iot_packet_verifier/Cargo.toml ./iot_packet_verifier/Cargo.toml
COPY iot_verifier/Cargo.toml ./iot_verifier/Cargo.toml
COPY poc_entropy/Cargo.toml ./poc_entropy/Cargo.toml
COPY reward_index/Cargo.toml ./reward_index/Cargo.toml

# Enable sparse registry to avoid crates indexing infinite loop
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN bash -c 'mkdir {ingest,iot_config,iot_packet_verifier,iot_verifier,poc_entropy,reward_index}/src' \
 # Create a dummy project files to build deps around
 && echo "fn main() {}" > ./ingest/src/main.rs \
 && echo "fn main() {}" > ./iot_config/src/main.rs \
 && echo "fn main() {}" > ./iot_packet_verifier/src/main.rs \
 && echo "fn main() {}" > ./iot_verifier/src/main.rs \
 && echo "fn main() {}" > ./poc_entropy/src/main.rs \
 && echo "fn main() {}" > ./reward_index/src/main.rs \
 # Remove unused workspace members to avoid compile error on missing members
 && sed -i -e '/mobile_config_cli/d' \
           -e '/mobile_config/d' \
           -e '/mobile_packet_verifier/d' \
           -e '/mobile_verifier/d' \
           Cargo.toml \
 && cargo build --release

# Compile the ingest service
FROM dependencies as builder-ingest
COPY ingest ./ingest/
RUN cargo build --package ingest --release

# Compile poc-entropy service
FROM dependencies as builder-entropy 
COPY poc_entropy ./poc_entropy/
RUN cargo build --package poc-entropy --release

# Compile reward-index service
FROM dependencies as builder-rewarder
COPY reward_index ./reward_index/
RUN cargo build --package reward-index --release

# Compile the iot-config service
FROM dependencies as builder-config
COPY iot_config ./iot_config/
RUN cargo build --package iot-config --release

# Compile iot-packet-verifier service
FROM builder-config as builder-packet-verifier
COPY iot_packet_verifier ./iot_packet_verifier/
RUN cargo build --package iot-packet-verifier --release

# Compile iot-verifier service
FROM builder-config as builder-verifier
COPY iot_verifier ./iot_verifier/
RUN cargo build --package iot-verifier --release

# Wrap it all up with a bow
FROM debian:bullseye-slim

COPY --from=builder-ingest ./target/release/ingest /opt/ingest/bin/ingest
COPY --from=builder-entropy ./target/release/poc-entropy /opt/poc-entropy/bin/poc-entropy
COPY --from=dependencies ./target/release/price /opt/price/bin/price
COPY --from=builder-config ./target/release/iot-config /opt/iot-config/bin/iot-config
COPY --from=builder-packet-verifier ./target/release/iot-packet-verifier /opt/iot-packet-verifier/bin/iot-packet-verifier
COPY --from=builder-verifier ./target/release/iot-verifier /opt/iot-verifier/bin/iot-verifier
COPY --from=builder-rewarder ./target/release/reward-index /opt/reward-index/bin/reward-index
