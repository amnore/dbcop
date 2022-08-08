#!/bin/bash
set -ex
set -o pipefail

SESSIONS=(25)
TXNS_PER_SESSION=(400)
OPS_PER_TXN=(8)
VARIABLES=(10000)
READ_PROBABILITY=(0.3 0.5 0.95)
KEY_DISTRIBUTION=("uniform")

HISTORIES=3
PARAMS=()

GENERATE_DEST=/tmp/generate/
HIST_DEST=/tmp/history/
SI_DEST=/tmp/si/
SI_NO_COALESCING_DEST=/tmp/si-no-coalescing/
SI_NO_PRUNING_DEST=/tmp/si-no-pruning/
SI_NO_PRUNING_COALESCING_DEST=/tmp/si-no-pruning-coalescing/
CSV_DEST=/tmp/csv

DB=postgres
ADDR=127.0.0.1:5432

SI_DIR="$HOME/Source/CobraVerifier"
GENERATOR_DIR="$HOME/Source/generator"

declare -A AVG_TIME_SI=()
declare -A AVG_TIME_SI_NO_COALESCING=()
declare -A AVG_TIME_SI_NO_PRUNING=()
declare -A AVG_TIME_SI_NO_PRUNING_COALESCING=()

# build tools
cargo build --manifest-path "$GENERATOR_DIR/Cargo.toml" --release
# cargo build --manifest-path "$GENERATOR_DIR/Cargo.toml" --release --example $DB

#rm -rf $GENERATE_DEST $HIST_DEST $SI_DEST $SI_NO_COALESCING_DEST $SI_NO_PRUNING_DEST $SI_NO_PRUNING_COALESCING_DEST $CSV_DEST

# generate operations
for i in "${SESSIONS[@]}"; do
  for j in "${TXNS_PER_SESSION[@]}"; do
    for k in "${OPS_PER_TXN[@]}"; do
      for l in "${VARIABLES[@]}"; do
        for m in "${READ_PROBABILITY[@]}"; do
          for n in "${KEY_DISTRIBUTION[@]}"; do
            PARAMS+=("${i}_${j}_${k}_${l}_${m}_${n}")
#            mkdir -p "$GENERATE_DEST/${i}_${j}_${k}_${l}_${m}_${n}"
#            "$GENERATOR_DIR/target/release/dbcop" generate -d "/tmp/generate/${i}_${j}_${k}_${l}_${m}_${n}" --nhist $HISTORIES -n $i -t $j -e $k -v $l --readp $m --key_distrib $n
          done
        done
      done
    done
  done
done

# run operations to get history
for p in "${PARAMS[@]}"; do
  mkdir -p "$HIST_DEST/$p"
#  "$GENERATOR_DIR/target/release/dbcop" run $ADDR --db $DB --dir "/tmp/generate/$p" --out "$HIST_DEST/$p" >/dev/null
done

# verify with si
#for p in "${PARAMS[@]}"; do
#  mkdir -p "$SI_DEST/$p"
#  for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
#    timeout 180 java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit -t dbcop "$hist/history.bincode" &> "${hist/$HIST_DEST/$SI_DEST}" || true
#  done
#done

# verify with si (no pruning)
for p in "${PARAMS[@]}"; do
  mkdir -p "$SI_NO_PRUNING_DEST/$p"
  for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
    timeout 180 java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit -t dbcop --no-pruning "$hist/history.bincode" &> "${hist/$HIST_DEST/$SI_NO_PRUNING_DEST}" || true
  done
done

# verify with si (no coalescing)
#for p in "${PARAMS[@]}"; do
#  mkdir -p "$SI_NO_COALESCING_DEST/$p"
#  for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
#    timeout 180 java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit -t dbcop --no-coalescing "$hist/history.bincode" &> "${hist/$HIST_DEST/$SI_NO_COALESCING_DEST}" || true
#  done
#done

# verify with si (no pruning, coalescing)
for p in "${PARAMS[@]}"; do
  mkdir -p "$SI_NO_PRUNING_COALESCING_DEST/$p"
  for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
    timeout 180 java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit -t dbcop --no-pruning --no-coalescing "$hist/history.bincode" &> "${hist/$HIST_DEST/$SI_NO_PRUNING_COALESCING_DEST}" || true
  done
done

# compute average time
for p in "${PARAMS[@]}"; do
  time=()
  for hist in $(find "$SI_DEST/$p" -name "hist-*"); do
    n="$(cat $hist | sed -nE 's/^ENTIRE_EXPERIMENT: ([[:digit:]]+)ms$/\1/p')"
    if [ -n "$n" ]; then
      time+=("$(bc <<<"scale=4; $n / 1000")")
    else
      time+=(180)
    fi
  done
  AVG_TIME_SI[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

  time=()
  for hist in $(find "$SI_NO_PRUNING_DEST/$p" -name "hist-*"); do
    n="$(cat $hist | sed -nE 's/^ENTIRE_EXPERIMENT: ([[:digit:]]+)ms$/\1/p')"
    if [ -n "$n" ]; then
      time+=("$(bc <<<"scale=4; $n / 1000")")
    else
      time+=(180)
    fi
  done
  AVG_TIME_SI_NO_PRUNING[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

  time=()
  for hist in $(find "$SI_NO_COALESCING_DEST/$p" -name "hist-*"); do
    n="$(cat $hist | sed -nE 's/^ENTIRE_EXPERIMENT: ([[:digit:]]+)ms$/\1/p')"
    if [ -n "$n" ]; then
      time+=("$(bc <<<"scale=4; $n / 1000")")
    else
      time+=(180)
    fi
  done
  AVG_TIME_SI_NO_COALESCING[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

  time=()
  for hist in $(find "$SI_NO_PRUNING_COALESCING_DEST/$p" -name "hist-*"); do
    n="$(cat $hist | sed -nE 's/^ENTIRE_EXPERIMENT: ([[:digit:]]+)ms$/\1/p')"
    if [ -n "$n" ]; then
      time+=("$(bc <<<"scale=4; $n / 1000")")
    else
      time+=(180)
    fi
  done
  AVG_TIME_SI_NO_PRUNING_COALESCING[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"
done

mkdir -p $CSV_DEST
echo "param,si,si(no-pruning),si(no-coalescing),si(no-pruning-coalescing)" > "$CSV_DEST/avg_time.csv"
for p in "${PARAMS[@]}"; do
  echo "$p,${AVG_TIME_SI[$p]},${AVG_TIME_SI_NO_PRUNING[$p]},${AVG_TIME_SI_NO_COALESCING[$p]},${AVG_TIME_SI_NO_PRUNING_COALESCING[$p]}" >> "$CSV_DEST/avg_time.csv"
done
