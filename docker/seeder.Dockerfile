FROM rust:1.72

ENV STACK=iot
ENV DB_URL=postgres://postgres:postgres@postgres:5432

RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres

COPY ./iot_config/migrations /iot_config/migrations
COPY ./iot_packet_verifier/migrations /iot_packet_verifier/migrations
COPY ./iot_verifier/migrations /iot_verifier/migrations
COPY ./mobile_config/migrations /mobile_config/migrations
COPY ./mobile_packet_verifier/migrations /mobile_packet_verifier/migrations
COPY ./mobile_verifier/migrations /mobile_verifier/migrations
COPY ./reward_index/migrations /reward_index/migrations

RUN <<EOF
echo "#!/bin/bash" >> /run_migrations
echo "if [ \"${STACK}\" == \"iot\" ]; then" >> /run_migrations
echo "    echo \"Running iot-config migrations\"" >> /run_migrations
echo "    sqlx migrate run --database-url ${DB_URL}/iot_config_db --source /iot_config/migrations" >> /run_migrations
echo "    echo \"Running iot-packet-verifier migrations\"" >> /run_migrations
echo "    sqlx migrate run --database-url ${DB_URL}/iot_packet_verifier_db --source /iot_packet_verifier/migrations" >> /run_migrations
echo "    echo \"Running iot-verifier migrations\"" >> /run_migrations
echo "    sqlx migrate run --database-url ${DB_URL}/iot_verifier_db --source /iot_verifier/migrations" >> /run_migrations
echo "    echo \"Running iot reward-index migrations\"" >> /run_migrations
echo "    sqlx migrate run --database-url ${DB_URL}/iot_index_db --source /reward_index/migrations" >> /run_migrations
echo "elif [ \"${STACK}\" == \"mobile\" ]; then" >> /run_migrations
echo "    echo \"Running mobile-config migrations\"" >> /run_migrations
echo "    sqlx migrate run --database-url ${DB_URL}/mobile_config_db --source /mobile_config/migrations" >> /run_migrations
echo "    echo \"Running mobile-packet-verifier migrations\"" >> /run_migrations
echo "    sqlx migrate run --database-url ${DB_URL}/mobile_packet_verifier_db --source /mobile_packet_verifier/migrations" >> /run_migrations
echo "    echo \"Running mobile-verifier migrations\"" >> /run_migrations
echo "    sqlx migrate run --database-url ${DB_URL}/mobile_verifier_db --source /mobile_verifier/migrations" >> /run_migrations
echo "    echo \"Running mobile reward-index migrations\"" >> /run_migrations
echo "    sqlx migrate run --database-url ${DB_URL}/mobile_index_db --source /reward_index/migrations" >> /run_migrations
echo "else" >> /run_migrations
echo "    echo \"invalid stack selected; must one of 'iot' or 'mobile'\"" >> /run_migrations
echo "fi" >> /run_migrations
echo "echo \"Finished running all migrations\"" >> /run_migrations
EOF
RUN chmod a+x /run_migrations

CMD ["/run_migrations"]