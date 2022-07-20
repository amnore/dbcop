#!/bin/sh
set -ex pipefail

SI=~/Source/CobraVerifier
HIST=~/Downloads/DBCopExecutions
DEST=/tmp/si-dbcop

rm -rf $DEST

for d in $(ls "$HIST"); do
  for p in $(ls "$HIST/$d"); do
    mkdir -p "$DEST/$d/$p"

    for h in $(ls "$HIST/$d/$p"); do
      java -jar "$SI/build/libs/CobraVerifier-0.0.1-SNAPSHOT.jar" audit -t dbcop "$HIST/$d/$p/$h/history.bincode" &> "$DEST/$d/$p/$h" || true
    done
  done
done
