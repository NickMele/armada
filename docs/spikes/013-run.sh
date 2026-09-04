#!/bin/zsh
# Spike 13 — what one gate's reading of the worktree costs.
#
# Builds `013-measure.rs` against this workspace's own adapter, then times
# `WorkProduct::changed_files` over three worktrees: one with nothing changed,
# one with a step-sized diff, one with a diff far larger than a step's.
#
# Run from a checkout of this repository. WORKTREE is a linked worktree of it —
# a Drone's shape, build directory and all — and SHARED is the ordinary
# checkout, whose branch is its own diff base and so answers with nothing
# changed. Both are read, neither is written, except for the scratch directory
# the large scenario makes and removes.
#
# The machine's load average is recorded on every row, because it moves the
# answer more than the diff does and Fleet runs several Drones at once.
set -e

REPO=${REPO:?the repository this is run against}
WORKTREE=${WORKTREE:?a linked worktree of it}
BRANCH=${BRANCH:?the branch checked out in that worktree}
SHARED=$REPO
OUT=${OUT:-013-samples.csv}
N=${N:-20}
BATCHES=${BATCHES:-4}

BUILD=$(mktemp -d)
mkdir -p $BUILD/src
cp $(dirname $0)/013-measure.rs $BUILD/src/main.rs
cat > $BUILD/Cargo.toml <<TOML
[package]
name = "measure-changed-files"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[dependencies]
adapters = { path = "$REPO/crates/adapters" }
adapter-traits = { path = "$REPO/crates/adapter-traits" }
core-model = { path = "$REPO/crates/core-model" }
verification = { path = "$REPO/crates/verification" }
TOML
(cd $BUILD && cargo build --release)
BIN=$BUILD/target/release/measure-changed-files

echo "scenario,call,micros,files,scan_micros,loadavg" > $OUT
for batch in $(seq 1 $BATCHES); do
  LOAD=$(uptime | sed 's/.*load averages*: //' | awk '{print $1}')
  for scenario in none realistic large; do
    case $scenario in
      none)      P=$SHARED; B=$(git -C $SHARED branch --show-current) ;;
      realistic) P=$WORKTREE; B=$BRANCH ;;
      large)     P=$WORKTREE; B=$BRANCH ;;
    esac
    if [ "$scenario" = "large" ]; then
      mkdir -p $WORKTREE/.measure-tmp
      for i in $(seq 1 400); do
        printf 'line one\nline two\nline three %s\n' "$i" > $WORKTREE/.measure-tmp/file-$i.txt
      done
    else
      rm -rf $WORKTREE/.measure-tmp
    fi
    $BIN $P $B $N $scenario 2>>${OUT%.csv}-summaries.txt | sed "s/\$/,${LOAD}/" >> $OUT
  done
  rm -rf $WORKTREE/.measure-tmp
done
