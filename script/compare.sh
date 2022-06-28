#!/bin/bash
set -ex
set -o pipefail

SESSIONS=25
HISTORIES=5
TXNS_PER_SESSION=(10 20 30 40 50 100 200 300 400 500)
OPS_PER_TXN=(20)
VARIABLES=(10000)
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

declare -A AVG_TIME_COBRA=()
declare -A AVG_TIME_SI=()
declare -A AVG_TIME_DBCOP=()

# build tools
# cargo build --manifest-path "$DBCOP_DIR/Cargo.toml" --release
# cargo build --manifest-path "$DBCOP_DIR/Cargo.toml" --release --example $DB

# rm -rf $GENERATE_DEST $HIST_DEST $COBRA_DEST $DBCOP_DEST $SI_DEST

# generate operations
for i in "${TXNS_PER_SESSION[@]}"; do
  for j in "${OPS_PER_TXN[@]}"; do
    for k in "${VARIABLES[@]}"; do
      PARAMS+=("${i}_${j}_${k}")
      # mkdir -p "$GENERATE_DEST/${i}_${j}_${k}"
      # "$DBCOP_DIR/target/release/dbcop" generate -d "/tmp/generate/${i}_${j}_${k}" -h $HISTORIES -n $SESSIONS -t $i -e $j -v $k
    done
  done
done

# # run operations to get history
# for p in "${PARAMS[@]}"; do
  # mkdir -p "$HIST_DEST/$p"
  # "$DBCOP_DIR/target/release/examples/$DB" $ADDR --dir "/tmp/generate/$p" --out "$HIST_DEST/$p" >/dev/null
# done

# # verify with si
# for p in "${PARAMS[@]}"; do
  # mkdir -p "$SI_DEST/$p"
  # for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
    # java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit -t dbcop "$hist/history.bincode" &> "${hist/$HIST_DEST/$SI_DEST}"
  # done
# done

# # verify with cobra (transformed with si2ser)
# for p in "${PARAMS[@]}"; do
  # mkdir -p "$COBRA_DEST/$p"
  # for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
    # SI2SER_HIST="${hist/$HIST_DEST/$HIST_SI2SER_DEST}"
    # mkdir -p $SI2SER_HIST
    # java -jar "$SI_DIR/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" convert -f dbcop -o cobra -t si2ser $hist/history.bincode $SI2SER_HIST
    # java "-Djava.library.path=$COBRA_DIR/include/:$COBRA_DIR/build/monosat" -jar "$COBRA_DIR/target/CobraVerifier-0.0.1-SNAPSHOT-jar-with-dependencies.jar" mono audit "$HOME/Source/CobraVerifier/cobra.conf.default" $SI2SER_HIST &> "${hist/$HIST_DEST/$COBRA_DEST}"
  # done
# done

# # verify with dbcop (timeout 3m)
# for p in "${PARAMS[@]}"; do
  # mkdir -p "$DBCOP_DEST/$p"
  # for hist in $(find "$HIST_DEST/$p" -name "hist-*"); do
    # timeout 180 ./target/release/dbcop verify --cons si --ver_dir $hist --out_dir "$DBCOP_DEST/$p" >/dev/null || true
    # mv "$DBCOP_DEST/$p/result_log.json" "$DBCOP_DEST/$p/$(basename $hist)"
  # done
# done

# compute average time
for p in "${PARAMS[@]}"; do
  time=()
  for hist in $(find "$COBRA_DEST/$p" -name "hist-*"); do
    n="$(cat $hist | sed -nE 's/^\[INFO \] >>> Overall runtime = ([[:digit:]]+)ms$/\1/p')"
    if [ -n "$n" ]; then
      time+=("$(bc <<<"scale=4; $n / 1000")")
    fi
  done
  if [ "${#time[@]}" -ne 0 ]; then
    AVG_TIME_COBRA[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"
  else
    AVG_TIME_COBRA[$p]="180"
  fi

  time=()
  for hist in $(find "$SI_DEST/$p" -name "hist-*"); do
    n="$(cat $hist | sed -nE 's/^ENTIRE_EXPERIMENT: ([[:digit:]]+)ms$/\1/p')"
    if [ -n "$n" ]; then
      time+=("$(bc <<<"scale=4; $n / 1000")")
    fi
  done
  if [ "${#time[@]}" -ne 0 ]; then
    AVG_TIME_SI[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"
  else
    AVG_TIME_SI[$p]="180"
  fi

  time=()
  for hist in $(find "$DBCOP_DEST/$p" -name "hist-*"); do
    n="$(cat $hist | sed -nE 's/.+"duration":([[:digit:]]+\.[[:digit:]]+).+/\1/p')"
    if [ -n "$n" ]; then
      time+=("$n")
    fi
  done
  if [ "${#time[@]}" -ne 0 ]; then
    AVG_TIME_DBCOP[$p]="$(IFS='+'; echo "scale=4; (${time[*]}) / ${#time[@]}" | bc)"
  else
    AVG_TIME_DBCOP[$p]="180"
  fi
done

cat <(echo "param cobra si dbcop") <(for p in "${PARAMS[@]}"; do echo "$p ${AVG_TIME_COBRA[$p]} ${AVG_TIME_SI[$p]} ${AVG_TIME_DBCOP[$p]}"; done) | column -t -s' '
