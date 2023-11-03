#!/usr/bin/env bash

set -eo pipefail

subcmd=$1;

build_seeder() {
  echo "building db seeder image"
  docker build -t oracle-db-seeder -f ./docker/seeder.Dockerfile .
  echo "seeder image complete"
}

# build_iot() {}

# build_mobile() {}

run_iot() {
  echo "starting iot compose stack"
  docker-compose -f ./docker/iot-compose.yml -p iot up -d
  echo "iot stack started"
}

run_mobile() {
  echo "starting mobile compose stack"
  docker-compose -f ./docker/mobile-compose.yml -p mobile up -d
  echo "mobile stack started"
}

stop_iot() {
  echo "stopping iot compose stack"
  docker-compose -f ./docker/iot-compose.yml -p iot down
  docker volume rm iot_db-data
  echo "iot stack stopped"
}

stop_mobile() {
  echo "stopping mobile compose stack"
  docker-compose -f ./docker/mobile-compose.yml -p mobile down
  docker volume rm mobile_db-data
  echo "mobile stack stopped"
}

build_iot_config() {
  echo "building iot-config service image"
  docker build -t iot-config -f ./docker/iot_config.Dockerfile .
  echo "finished building iot-config service image"
}

build_mobile_config() {
  echo "building mobile-config service image"
  docker build -t mobile-config -f ./docker/mobile_config.Dockerfile .
  echo "finished building mobile-config service image"
}

run_iot_config() {
  echo "starting iot-config and support services"
  docker-compose -f ./docker/config-svcs-compose.yml -p iot-config up -d iot-config
  echo "iot-config and support services started"
}

run_mobile_config() {
  echo "starting mobile-config and support services"
  docker-compose -f ./docker/config-svcs-compose.yml -p iot-config up -d mobile-config
  echo "mobile-config and support services started"
}

stop_iot_config() {
  echo "stopping iot config and related support services"
  docker-compose -f ./docker/config-svcs-compose.yml -p iot-config down
  echo "iot config and related support services stopped"
}

stop_mobile_config() {
  echo "stopping mobile config and related support services"
  docker-compose -f ./docker/config-svcs-compose.yml -p mobile-config down
  echo "mobile config and related support services stopped"
}

case $subcmd in
    build-seeder)
      build_seeder
      exit;;
    build-iot)
      exit;;
    build-mobile)
      exit;;
    run-iot)
      run_iot
      exit;;
    run-mobile)
      run_mobile
      exit;;
    stop-iot)
      stop_iot
      exit;;
    stop-mobile)
      stop_mobile
      exit;;
    build-iot-config)
      exit;;
    build-mobile-config)
      exit;;
    run-iot-config)
      run_iot_config
      exit;;
    run-mobile-config)
      run_mobile_config
      exit;;
    stop-iot-config)
      stop_iot_config
      exit;;
    stop-mobile-config)
      stop_mobile_config
      exit;;
    *)
      echo "subcommand must be one of:\n"
      echo "    build-seeder | build-iot | build-mobile | build-iot-config | build-mobile-config"
      echo "    run-iot | run-mobile | run-iot-config | run-mobile-config"
      echo "    stop-iot | stop-mobile | stop-iot-config | stop-mobile-config"
      exit;;
esac
