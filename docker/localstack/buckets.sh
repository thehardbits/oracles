#!/bin/bash

awslocal s3 mb s3://iot-ingest
awslocal s3 mb s3://iot-packet-ingest
awslocal s3 mb s3://iot-entropy
awslocal s3 mb s3://iot-verifier
awslocal s3 mb s3://iot-packet-verifier
awslocal s3 mb s3://iot-price
awslocal s3 mb s3://iot-rewards
awslocal s3 mb s3://mobile-ingest
awslocal s3 mb s3://mobile-packet-ingest
awslocal s3 mb s3://mobile-verifier
awslocal s3 mb s3://mobile-packet-verifier
awslocal s3 mb s3://mobile-price
awslocal s3 mb s3://mobile-rewards