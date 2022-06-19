#!/bin/bash
set -ex

SESSIONS=25
HISTORIES=10
TXNS_PER_SESSION=(50 100 200 300 400 500 600)
OPS_PER_TXN=(10 20 30)
VARIABLES=(1000 2000 4000 8000 10000)
PARAMS=()

GENERATE_DEST=/tmp/generate/
HIST_DEST=/tmp/history/
HIST_SI2SER_DEST=/tmp/history_ser/
COBRA_DEST=/tmp/cobra/
DBCOP_DEST=/tmp/dbcop/
SI_DEST=/tmp/si/

DB=postgres
ADDR=127.0.0.1:5432

COBRA_DIR="$HOME/Source/CobraVerifier"
SI_DIR="$HOME/Source/CobraVerifier"
DBCOP_DIR="$HOME/Source/dbcop"

# build tools
cargo build --manifest-path "$DBCOP_DIR/Cargo.toml" --release
cargo build --manifest-path "$DBCOP_DIR/Cargo.toml" --release --example $DB

rm -rf $GENERATE_DEST $HIST_DEST $COBRA_DEST $DBCOP_DEST $SI_DEST

# generate operations
for i in $TXNS_PER_SESSION; do
  for j in $OPS_PER_TXN; do
    for k in $VARIABLES; do
      PARAMS+="${i}_${j}_${k}"
      mkdir -p "$GENERATE_DEST/${i}_${j}_${k}"
      "$DBCOP_DIR/target/release/dbcop" generate -d "/tmp/generate/${i}_${j}_${k}" -h $HISTORIES -n $SESSIONS -t $i -e $j -v $k;
    done
  done
done

# run operations to get history
for p in $PARAMS; do
  mkdir -p "$HIST_DEST/$p"
  "$DBCOP_DIR/target/release/examples/$DB" $ADDR --dir "/tmp/generate/$p" --out "$HIST_DEST/$p"
done

# verify with si
for p in $PARAMS; do
  mkdir -p "$SI_DEST/$p"
  for hist in $(find "$HIST_DEST/$p" -type f); do
    java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit -t dbcop $hist &> "${hist/$HIST_DEST/$SI_DEST}"
  done
done

# verify with cobra (transformed with si2ser)
for p in $PARAMS; do
  mkdir -p "$COBRA_DEST/$p" "$HIST_SI2SER_DEST/$p"
  for hist in $(find "$HIST_DEST/$p" -type f); do
    java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" convert -f dbcop -o cobra -t si2ser $hist "${hist/$HIST_DEST/$HIST_SI2SER_DEST}"
    java "-Djava.library.path=$COBRA_DIR/include/:$COBRA_DIR/build/monosat" -jar "$COBRA_DIR/target/CobraVerifier-0.0.1-SNAPSHOT-jar-with-dependencies.jar" mono audit "$HOME/Source/CobraVerifier/cobra.conf.default" "${hist/$HIST_DEST/HIST_SI2SER_DEST}" &> "${hist/$HIST_DEST/$COBRA_DEST}"
  done
done

# verify with dbcop (timeout 3m)
for p in $PARAMS; do
  mkdir -p "$DBCOP_DEST/$p"
  for hist in $(find "$HIST_DEST/$p" -type f); do
    ./target/release/dbcop verify --cons si --ver_dir "$HIST_DEST/$p" --out_dir "$DBCOP_DEST/$p" &
    pid=$!
    sleep 180 && kill $pid &>/dev/null &
    wait $pid
  done
done
