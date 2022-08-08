#!/bin/sh
set -ex
set -o pipefail

HIST_DEST=~/Source/CobraLogs/one-shot-10k
# HIST_DEST=~/Source/CobraLogs/one-shot-chengRW
DATASETS=()

for hist in $(ls $HIST_DEST); do
  DATASETS+=($hist)
done

HIST_COBRA_SI_DEST=/tmp/history_cobra_si/
HIST_COBRA_DEST=/tmp/history_cobra/
HIST_DBCOP_DEST=/tmp/history_dbcop/
COBRA_DEST=/tmp/cobra/
COBRA_SI_DEST=/tmp/cobra_si/
COBRA_NOGPU_DEST=/tmp/cobra_nogpu/
SI_DEST=/tmp/si/
CSV_DEST=/tmp/csv
DBCOP_DEST=/tmp/dbcop/

COBRA_DIR="$HOME/Source/CobraVerifier"
SI_DIR="$HOME/Source/CobraVerifier"
DBCOP_DIR="$HOME/Source/dbcop"

declare -A AVG_TIME_COBRA=()
declare -A AVG_TIME_SI=()
declare -A AVG_TIME_COBRA_SI=()
declare -A AVG_TIME_COBRA_NOGPU=()
declare -A AVG_TIME_DBCOP=()

rm -rf $HIST_COBRA_SI_DEST $HIST_COBRA_DEST $COBRA_DEST $COBRA_SI_DEST $COBRA_NOGPU_DEST $SI_DEST $CSV_DEST
mkdir -p $HIST_COBRA_SI_DEST $HIST_COBRA_DEST $COBRA_DEST $COBRA_SI_DEST $COBRA_NOGPU_DEST $SI_DEST $CSV_DEST

# verify with si
# for p in "${DATASETS[@]}"; do
  # hist="$HIST_DEST/$p" 
  # java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit "$hist" &> "${hist/$HIST_DEST/$SI_DEST}"
# done

# # verify with cobra (transformed with si2ser)
# for p in "${DATASETS[@]}"; do
  # hist="$HIST_DEST/$p"
  # SI2SER_HIST="${hist/$HIST_DEST/$HIST_COBRA_SI_DEST}"
  # mkdir -p $SI2SER_HIST
  # java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" convert -f cobra -o cobra -t si2ser $hist $SI2SER_HIST
  # timeout 180 java -Xmx8g "-Djava.library.path=$COBRA_DIR/include/:$COBRA_DIR/build/monosat" -jar "$COBRA_DIR/target/CobraVerifier-0.0.1-SNAPSHOT-jar-with-dependencies.jar" mono audit "$HOME/Source/CobraVerifier/cobra.conf.default" $SI2SER_HIST &> "${hist/$HIST_DEST/$COBRA_SI_DEST}" || true
# done

# verify with cobra (original)
# for p in "${DATASETS[@]}"; do
  # hist="$HIST_DEST/$p"
  # java "-Djava.library.path=$COBRA_DIR/include/:$COBRA_DIR/build/monosat" -jar "$COBRA_DIR/target/CobraVerifier-0.0.1-SNAPSHOT-jar-with-dependencies.jar" mono audit "$HOME/Source/CobraVerifier/cobra.conf.default" $hist &> "${hist/$HIST_DEST/$COBRA_DEST}"
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
for p in "${DATASETS[@]}"; do
  hist="$HIST_DEST/$p"
  timeout 600 java "-Djava.library.path=$COBRA_DIR/include/:$COBRA_DIR/build/monosat" -jar "$COBRA_DIR/target/CobraVerifier-0.0.1-SNAPSHOT-jar-with-dependencies.jar" mono audit /tmp/cobra.conf.nogpu $hist &> "${hist/$HIST_DEST/$COBRA_NOGPU_DEST}" || true
done

# verify with dbcop (timeout 3m)
# for p in "${DATASETS[@]}"; do
  # cobra_hist="$HIST_DEST/$p"
  # hist="$HIST_DBCOP_DEST/$p"
  # mkdir -p "$hist"
  # java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" convert -f cobra -o dbcop "$cobra_hist" "$hist/history.bincode"
  # timeout 180 $DBCOP_DIR/target/release/dbcop verify --cons ser --ver_dir $hist --out_dir "$DBCOP_DEST" >/dev/null || true
  # mv "$DBCOP_DEST/result_log.json" "$DBCOP_DEST/$p"
# done

# compute average time
for p in "${DATASETS[@]}"; do
  time=()
  hist="$COBRA_NOGPU_DEST/$p"
  n="$(cat $hist | sed -nE 's/^\[INFO \] >>> Overall runtime = ([[:digit:]]+)ms$/\1/p')"
  if [ -n "$n" ]; then
    time+=("$(bc <<<"scale=4; $n / 1000")")
  else
    time+=(180)
  fi
  AVG_TIME_COBRA_NOGPU[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

#  time=()
#  hist="$COBRA_DEST/$p" 
#  n="$(cat $hist | sed -nE 's/^\[INFO \] >>> Overall runtime = ([[:digit:]]+)ms$/\1/p')"
#  if [ -n "$n" ]; then
#    time+=("$(bc <<<"scale=4; $n / 1000")")
#  else
#    time+=(180)
#  fi
#  AVG_TIME_COBRA[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

#  time=()
#  hist="$COBRA_SI_DEST/$p" 
#  n="$(cat $hist | sed -nE 's/^\[INFO \] >>> Overall runtime = ([[:digit:]]+)ms$/\1/p')"
#  if [ -n "$n" ]; then
#    time+=("$(bc <<<"scale=4; $n / 1000")")
#  else
#    time+=(180)
#  fi
#  AVG_TIME_COBRA_SI[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

#  time=()
#  hist="$SI_DEST/$p" 
#  n="$(cat $hist | sed -nE 's/^ENTIRE_EXPERIMENT: ([[:digit:]]+)ms$/\1/p')"
#  if [ -n "$n" ]; then
#    time+=("$(bc <<<"scale=4; $n / 1000")")
#  else
#    time+=(180)
#  fi
#  AVG_TIME_SI[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

#  time=()
#  hist="$DBCOP_DEST/$p"
#  n="$(cat $hist | sed -nE 's/.+"duration":([[:digit:]]+\.[[:digit:]]+).+/\1/p')"
#  if [ -n "$n" ]; then
#    time+=("$n")
#  else
#    time+=(180)
#  fi
#  AVG_TIME_DBCOP[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"
done

mkdir -p $CSV_DEST
echo "param,cobra,cobra(si),cobra(nogpu),si,oopsla" > "$CSV_DEST/avg_time.csv"
for p in "${DATASETS[@]}"; do
  echo "$p,${AVG_TIME_COBRA[$p]},${AVG_TIME_COBRA_SI[$p]},${AVG_TIME_COBRA_NOGPU[$p]},${AVG_TIME_SI[$p]},${AVG_TIME_DBCOP[$p]}" >> "$CSV_DEST/avg_time.csv"
done

