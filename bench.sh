#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

outroot=runs
runname="$1"
run="$(date -Is)-$runname"
rundir=$outroot/$run
flamegraph=~/.local/lib/FlameGraph
if [[ ! -f $flamegraph/flamegraph.pl ]]; then
    echo flamegraph not found in $flamegraph
    exit 1
fi

cargo build --release
cargo build -p integration_testing --release

echo "run: $run"
pushd docker
export DOCKER_USER=$(id -u):$(id -g) 
docker compose down
rm -rf volumes/postgres
mkdir -p volumes/postgres
docker compose up postgres --wait
popd

mkdir -p $rundir
perffile=$rundir/perf.data
RUST_LOG=warn LEMMY_ALLOW_HTTP=1 perf record --freq=99 --output $perffile --call-graph dwarf -- \
    target/release/lemmy_server > $rundir/server.log &
SERVERPID=$!
trap "kill $SERVERPID" EXIT

while ! curl localhost:8536/api/v3/site >/dev/null 2>&1; do
    echo waiting for server up
    sleep 1
done
echo server up


RUST_LOG=warn target/release/integration_testing --remote-server http://lemmy.localhost:8536/inbox \
    --input-dir ~/reddit --limit 100000 --skip 0 --output-json $rundir/info.json \
    --runname $runname \
    > $rundir/upload.log

sleep 3
kill $SERVERPID 2>/dev/null || true
# remove trap
trap - EXIT

wait $SERVERPID || true

perfscript=$perffile.txt
perf script -i $perffile -F +pid > $perfscript
$flamegraph/stackcollapse-perf.pl $perfscript | $flamegraph/flamegraph.pl --title "$run" --subtitle lemmy_server --hash > $rundir/perf.data.flamegraph.svg
# perf data convert -i $perffile --to-json $perffile.json