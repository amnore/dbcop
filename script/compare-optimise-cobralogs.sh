#!/bin/sh
set -ex
set -o pipefail

HIST_DEST=~/Source/CobraLogs/one-shot-10k
# HIST_DEST=~/Source/CobraLogs/one-shot-chengRW
DATASETS=()

for hist in $(ls $HIST_DEST); do
  DATASETS+=($hist)
done

SI_DEST=/tmp/si/
SI_NO_COALESCING_DEST=/tmp/si-no-coalescing/
SI_NO_PRUNING_DEST=/tmp/si-no-pruning/
SI_NO_PRUNING_COALESCING_DEST=/tmp/si-no-pruning-coalescing/
CSV_DEST=/tmp/csv

SI_DIR="$HOME/Source/CobraVerifier"

declare -A AVG_TIME_SI=()
declare -A AVG_TIME_SI_NO_COALESCING=()
declare -A AVG_TIME_SI_NO_PRUNING=()
declare -A AVG_TIME_SI_NO_PRUNING_COALESCING=()

rm -rf $SI_DEST $SI_NO_COALESCING_DEST $SI_NO_PRUNING_DEST $SI_NO_PRUNING_COALESCING_DEST $CSV_DEST
mkdir -p $SI_DEST $SI_NO_COALESCING_DEST $SI_NO_PRUNING_DEST $SI_NO_PRUNING_COALESCING_DEST $CSV_DEST

# verify with si
for p in "${DATASETS[@]}"; do
  hist="$HIST_DEST/$p" 
  timeout 180 java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit "$hist" &> "${hist/$HIST_DEST/$SI_DEST}" || true
done

# verify with si (no coalescing)
for p in "${DATASETS[@]}"; do
  hist="$HIST_DEST/$p" 
  timeout 180 java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit --no-coalescing "$hist" &> "${hist/$HIST_DEST/$SI_NO_COALESCING_DEST}" || true
done

# verify with si (no pruning, coalescing)
for p in "${DATASETS[@]}"; do
  hist="$HIST_DEST/$p" 
  timeout 180 java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit --no-pruning --no-coalescing "$hist" &> "${hist/$HIST_DEST/$SI_NO_PRUNING_COALESCING_DEST}" || true
done

# verify with si (no pruning)
for p in "${DATASETS[@]}"; do
  hist="$HIST_DEST/$p" 
  timeout 180 java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit --no-pruning "$hist" &> "${hist/$HIST_DEST/$SI_NO_PRUNING_DEST}" || true
done

# compute average time
for p in "${DATASETS[@]}"; do
  time=()
  hist="$SI_DEST/$p" 
  n="$(cat $hist | sed -nE 's/^ENTIRE_EXPERIMENT: ([[:digit:]]+)ms$/\1/p')"
  if [ -n "$n" ]; then
    time+=("$(bc <<<"scale=4; $n / 1000")")
  else
    time+=(180)
  fi
  AVG_TIME_SI[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

  time=()
  hist="$SI_NO_PRUNING_DEST/$p" 
  n="$(cat $hist | sed -nE 's/^ENTIRE_EXPERIMENT: ([[:digit:]]+)ms$/\1/p')"
  if [ -n "$n" ]; then
    time+=("$(bc <<<"scale=4; $n / 1000")")
  else
    time+=(180)
  fi
  AVG_TIME_SI_NO_PRUNING[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

  time=()
  hist="$SI_NO_COALESCING_DEST/$p" 
  n="$(cat $hist | sed -nE 's/^ENTIRE_EXPERIMENT: ([[:digit:]]+)ms$/\1/p')"
  if [ -n "$n" ]; then
    time+=("$(bc <<<"scale=4; $n / 1000")")
  else
    time+=(180)
  fi
  AVG_TIME_SI_NO_COALESCING[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"

  time=()
  hist="$SI_NO_PRUNING_COALESCING_DEST/$p" 
  n="$(cat $hist | sed -nE 's/^ENTIRE_EXPERIMENT: ([[:digit:]]+)ms$/\1/p')"
  if [ -n "$n" ]; then
    time+=("$(bc <<<"scale=4; $n / 1000")")
  else
    time+=(180)
  fi
  AVG_TIME_SI_NO_PRUNING_COALESCING[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"
done

mkdir -p $CSV_DEST
echo "param,si,si(no-pruning),si(no-coalescing),si(no-pruning-coalescing)" > "$CSV_DEST/avg_time.csv"
for p in "${DATASETS[@]}"; do
  echo "$p,${AVG_TIME_SI[$p]},${AVG_TIME_SI_NO_PRUNING[$p]},${AVG_TIME_SI_NO_COALESCING[$p]},${AVG_TIME_SI_NO_PRUNING_COALESCING[$p]}" >> "$CSV_DEST/avg_time.csv"
done
