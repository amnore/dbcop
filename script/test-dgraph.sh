#!/bin/sh
set -ex

DOCKER_COMPOSE_FILE=~/Source/dbcop/docker/dgraph/docker-compose.yml
DBCOP_PATH=~/Source/dbcop/
VERIFIER_PATH=~/Source/CobraVerifier

cargo build --manifest-path=$DBCOP_PATH/Cargo.toml --release 
cargo build --manifest-path=$DBCOP_PATH/Cargo.toml --release --example=dgraph
$VERIFIER_PATH/gradlew jar -p $VERIFIER_PATH

while true; do
  docker-compose -f $DOCKER_COMPOSE_FILE down
  docker-compose -f $DOCKER_COMPOSE_FILE up -d
  sleep 7
  rm -rf /tmp/generate /tmp/result
  mkdir -p /tmp/generate/
  $DBCOP_PATH/target/release/dbcop generate -d /tmp/generate/ -h 2 -n 25 -t 100 -e 20 -v 1000
  $DBCOP_PATH/target/release/examples/dgraph 127.0.0.1:9080 --dir /tmp/generate/ --out /tmp/result
  java -jar $VERIFIER_PATH/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar audit -t dbcop /tmp/result/hist-00000/history.bincode || break
  java -jar $VERIFIER_PATH/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar audit -t dbcop /tmp/result/hist-00001/history.bincode || break
done

rm -rf /tmp/violation && mkdir -p /tmp/violation/alpha /tmp/violation/zero
docker logs dgraph_alpha_1 &> /tmp/violation/alpha.log
docker logs dgraph_zero_1 &> /tmp/violation/zero.log
docker cp dgraph_alpha_1:. /tmp/violation/alpha
docker cp dgraph_zero_1:. /tmp/violation/zero
