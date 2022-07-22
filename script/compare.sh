#!/bin/bash
set -ex
set -o pipefail

# SESSIONS=(20)
# TXNS_PER_SESSION=(100)
# OPS_PER_TXN=(15)
# VARIABLES=(1000 2000 3000 4000 6000 8000 10000)
# READ_PROBABILITY=(0.5)
# KEY_DISTRIBUTION=("uniform")

# SESSIONS=(25)
# TXNS_PER_SESSION=(10 20 30 40 50 100 200 300 400 500)
# OPS_PER_TXN=(20)
# VARIABLES=(10000)
# READ_PROBABILITY=(0.5)
# KEY_DISTRIBUTION=("uniform")

SESSIONS=(20)
TXNS_PER_SESSION=(100)
OPS_PER_TXN=(5 10 15 20 25 30)
VARIABLES=(10000)
READ_PROBABILITY=(0.5)
KEY_DISTRIBUTION=("uniform")

# SESSIONS=(5 10 15 20 25 30 35)
# TXNS_PER_SESSION=(300)
# OPS_PER_TXN=(20)
# VARIABLES=(10000)
# READ_PROBABILITY=(0.5)
# KEY_DISTRIBUTION=("uniform")

# SESSIONS=(20)
# TXNS_PER_SESSION=(150)
# OPS_PER_TXN=(15)
# VARIABLES=(10000)
# READ_PROBABILITY=(0 0.25 0.5 0.75 1)
# KEY_DISTRIBUTION=("uniform")

# SESSIONS=(20)
# TXNS_PER_SESSION=(200)
# OPS_PER_TXN=(15)
# VARIABLES=(10000)
# READ_PROBABILITY=(0.5)
# KEY_DISTRIBUTION=("uniform" "zipf" "hotspot")

HISTORIES=3
PARAMS=()

GENERATE_DEST=/tmp/generate/
HIST_DEST=/tmp/history/
HIST_COBRA_SI_DEST=/tmp/history_cobra_si/
HIST_COBRA_DEST=/tmp/history_cobra/
COBRA_DEST=/tmp/cobra/
COBRA_SI_DEST=/tmp/cobra_si/
COBRA_SI_NOGPU_DEST=/tmp/cobra_si_nogpu/
COBRA_NOGPU_DEST=/tmp/cobra_nogpu/
DBCOP_DEST=/tmp/dbcop/
SI_DEST=/tmp/si/
CSV_DEST=/tmp/csv

DB=postgres
ADDR=127.0.0.1:5432

COBRA_DIR="$HOME/Source/CobraVerifier"
SI_DIR="$HOME/Source/CobraVerifier"
DBCOP_DIR="$HOME/Source/dbcop"
GENERATOR_DIR="$HOME/Source/generator"

declare -A AVG_TIME_COBRA=()
declare -A AVG_TIME_SI=()
declare -A AVG_TIME_DBCOP=()
declare -A AVG_TIME_COBRA_SI=()
declare -A AVG_TIME_COBRA_SI_NOGPU=()
declare -A AVG_TIME_COBRA_NOGPU=()

# build tools
cargo build --manifest-path "$GENERATOR_DIR/Cargo.toml" --release
cargo build --manifest-path "$GENERATOR_DIR/Cargo.toml" --release --example $DB

rm -rf $GENERATE_DEST $HIST_DEST $HIST_COBRA_SI_DEST $HIST_COBRA_DEST $COBRA_DEST $COBRA_SI_DEST $COBRA_SI_NOGPU_DEST $COBRA_NOGPU_DEST $DBCOP_DEST $SI_DEST $CSV_DEST

# generate operations
for i in "${SESSIONS[@]}"; do
  for j in "${TXNS_PER_SESSION[@]}"; do
    for k in "${OPS_PER_TXN[@]}"; do
      for l in "${VARIABLES[@]}"; do
        for m in "${READ_PROBABILITY[@]}"; do
          for n in "${KEY_DISTRIBUTION[@]}"; do
            PARAMS+=("${i}_${j}_${k}_${l}_${m}_${n}")
            mkdir -p "$GENERATE_DEST/${i}_${j}_${k}_${l}_${m}_${n}"
            "$GENERATOR_DIR/target/release/dbcop" generate -d "/tmp/generate/${i}_${j}_${k}_${l}_${m}_${n}" -h $HISTORIES -n $i -t $j -e $k -v $l --readp $m --key_distrib $n
          done
        done
      done
    done
  done
done

# run operations to get history
for p in "${PARAMS[@]}"; do
  mkdir -p "$HIST_DEST/$p"
  "$GENERATOR_DIR/target/release/examples/$DB" $ADDR --dir "/tmp/generate/$p" --out "$HIST_DEST/$p" >/dev/null
done

# verify with si
for p in "${PARAMS[@]}"; do
  mkdir -p "$SI_DEST/$p"
  for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
    java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit -t dbcop "$hist/history.bincode" &> "${hist/$HIST_DEST/$SI_DEST}"
  done
done

# verify with cobra (transformed with si2ser)
for p in "${PARAMS[@]}"; do
  mkdir -p "$COBRA_SI_DEST/$p"
  for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
    SI2SER_HIST="${hist/$HIST_DEST/$HIST_COBRA_SI_DEST}"
    mkdir -p $SI2SER_HIST
    java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" convert -f dbcop -o cobra -t si2ser $hist/history.bincode $SI2SER_HIST
    timeout 180 java "-Djava.library.path=$COBRA_DIR/include/:$COBRA_DIR/build/monosat" -jar "$COBRA_DIR/target/CobraVerifier-0.0.1-SNAPSHOT-jar-with-dependencies.jar" mono audit "$HOME/Source/CobraVerifier/cobra.conf.default" $SI2SER_HIST &> "${hist/$HIST_DEST/$COBRA_SI_DEST}" || true
  done
done

# verify with cobra (original)
# for p in "${PARAMS[@]}"; do
  # mkdir -p "$COBRA_DEST/$p"
  # for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
    # COBRA_HIST="${hist/$HIST_DEST/$HIST_COBRA_DEST}"
    # mkdir -p $COBRA_HIST
    # java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" convert -f dbcop -o cobra -t identity $hist/history.bincode $COBRA_HIST
    # java "-Djava.library.path=$COBRA_DIR/include/:$COBRA_DIR/build/monosat" -jar "$COBRA_DIR/target/CobraVerifier-0.0.1-SNAPSHOT-jar-with-dependencies.jar" mono audit "$HOME/Source/CobraVerifier/cobra.conf.default" $COBRA_HIST &> "${hist/$HIST_DEST/$COBRA_DEST}"
  # done
# done

cat > /tmp/cobra.conf.nogpu <<EOF
HEAVY_VALIDATION_CODE_ON=false
MULTI_THREADING_OPT=true
TIME_ORDER_ON=false
INFER_RELATION_ON=true
PCSG_ON=true
WRITE_SPACE_ON=true
MERGE_CONSTRAINT_ON=false
LOGGER_ON_SCREEN=true
LOGGER_PATH=/tmp/cobra/logger.log
LOGGER_LEVEL=INFO
LOG_FD_LOG=/tmp/cobra/log/
FETCHING_DURATION_BASE=500
FETCHING_DURATION_RAND=500
NUM_BATCH_FETCH_TRACE=1000
ONLINE_DB_TYPE=2
BENCH_TYPE=2
DUMP_POLYG=false
MAX_INFER_ROUNDS=1
BUNDLE_CONSTRAINTS=true
WW_CONSTRAINTS=true
BATCH_TX_VERI_SIZE=100
GPU_MATRIX=false
TOTAL_CLIENTS=24
DB_HOST=ye-cheng.duckdns.org
MIN_PROCESSING_NEW_TXN=5000
GC_EPOCH_THRESHOLD=100
TIME_DRIFT_THRESHOLD=100
EOF

# verify with cobra (nogpu, timeout 3m)
# for p in "${PARAMS[@]}"; do
  # mkdir -p "$COBRA_NOGPU_DEST/$p"
  # for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
    # COBRA_HIST="${hist/$HIST_DEST/$HIST_COBRA_DEST}"
    # timeout 180 java "-Djava.library.path=$COBRA_DIR/include/:$COBRA_DIR/build/monosat" -jar "$COBRA_DIR/target/CobraVerifier-0.0.1-SNAPSHOT-jar-with-dependencies.jar" mono audit /tmp/cobra.conf.nogpu $COBRA_HIST &> "${hist/$HIST_DEST/$COBRA_NOGPU_DEST}" || true
  # done
# done

# verify with dbcop (timeout 3m)
for p in "${PARAMS[@]}"; do
  mkdir -p "$DBCOP_DEST/$p"
  for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
    timeout 180 $DBCOP_DIR/target/release/dbcop verify --cons si --ver_dir $hist --out_dir "$DBCOP_DEST/$p" >/dev/null || true
    mv "$DBCOP_DEST/$p/result_log.json" "$DBCOP_DEST/$p/$(basename $hist)"
  done
done

# verify with cobra (transformed with si2ser, nogpu)
for p in "${PARAMS[@]}"; do
  mkdir -p "$COBRA_SI_NOGPU_DEST/$p"
  for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
    SI2SER_HIST="${hist/$HIST_DEST/$HIST_COBRA_SI_DEST}"
    timeout 180 java "-Djava.library.path=$COBRA_DIR/include/:$COBRA_DIR/build/monosat" -jar "$COBRA_DIR/target/CobraVerifier-0.0.1-SNAPSHOT-jar-with-dependencies.jar" mono audit "/tmp/cobra.conf.nogpu" $SI2SER_HIST &> "${hist/$HIST_DEST/$COBRA_SI_NOGPU_DEST}" || true
  done
done

# compute average time
for p in "${PARAMS[@]}"; do
  # time=()
  # for hist in $(find "$COBRA_NOGPU_DEST/$p" -name "hist-*"); do
    # n="$(cat $hist | sed -nE 's/^\[INFO \] >>> Overall runtime = ([[:digit:]]+)ms$/\1/p')"
    # if [ -n "$n" ]; then
      # time+=("$(bc <<<"scale=4; $n / 1000")")
    # else
      # time+=(180)
    # fi
  # done
  # AVG_TIME_COBRA_NOGPU[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

  # time=()
  # for hist in $(find "$COBRA_DEST/$p" -name "hist-*"); do
    # n="$(cat $hist | sed -nE 's/^\[INFO \] >>> Overall runtime = ([[:digit:]]+)ms$/\1/p')"
    # if [ -n "$n" ]; then
      # time+=("$(bc <<<"scale=4; $n / 1000")")
    # else
      # time+=(180)
    # fi
  # done
  # AVG_TIME_COBRA[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

  time=()
  for hist in $(find "$COBRA_SI_DEST/$p" -name "hist-*"); do
    n="$(cat $hist | sed -nE 's/^\[INFO \] >>> Overall runtime = ([[:digit:]]+)ms$/\1/p')"
    if [ -n "$n" ]; then
      time+=("$(bc <<<"scale=4; $n / 1000")")
    else
      time+=(180)
    fi
  done
  AVG_TIME_COBRA_SI[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

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
  for hist in $(find "$DBCOP_DEST/$p" -name "hist-*"); do
    n="$(cat $hist | sed -nE 's/.+"duration":([[:digit:]]+\.[[:digit:]]+).+/\1/p')"
    if [ -n "$n" ]; then
      time+=("$n")
    else
      time+=(180)
    fi
  done
  AVG_TIME_DBCOP[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

  time=()
  for hist in $(find "$COBRA_SI_NOGPU_DEST/$p" -name "hist-*"); do
    n="$(cat $hist | sed -nE 's/^\[INFO \] >>> Overall runtime = ([[:digit:]]+)ms$/\1/p')"
    if [ -n "$n" ]; then
      time+=("$(bc <<<"scale=4; $n / 1000")")
    else
      time+=(180)
    fi
  done
  AVG_TIME_COBRA_SI_NOGPU[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"
done

mkdir -p $CSV_DEST
echo "param,cobra(si),cobra(si-nogpu),si,oopsla" > "$CSV_DEST/avg_time.csv"
for p in "${PARAMS[@]}"; do
  echo "$p,${AVG_TIME_COBRA_SI[$p]},${AVG_TIME_COBRA_SI_NOGPU[$p]},${AVG_TIME_SI[$p]},${AVG_TIME_DBCOP[$p]}" >> "$CSV_DEST/avg_time.csv"
done
