#!/bin/sh
# Files development record files into the card statistics, staging's database and then
# production's (R1933001), as totals only (`jackioh-server stats-import --totals-only`, R1976000).
# gather.yml calls it after each batch it plays, the learn lane's shards after each piece of an
# export (selfplay-shards.yml), and learn.yml's import once the retrain has ended.
#
#   sh training/file-records.sh <jackioh-server> <records.jsonl>...
#
# Each file goes into each database whose address is set: STATS_IMPORT_DATABASE_URL (staging's) and
# PRODUCTION_STATS_IMPORT_DATABASE_URL (production's); with neither, it files nothing. Each database
# claims a file once by the SHA-256 of its contents, so filing the same file again folds nothing:
# a rerun, or the import after the shards, files only what a database is missing. An empty file is
# skipped. It stops at the first file a database refuses or cannot take, and exits non-zero.
set -eu

[ "$#" -ge 2 ] || {
  echo "usage: sh training/file-records.sh <jackioh-server> <records.jsonl>..." >&2
  exit 2
}
server=$1
shift
for file in "$@"; do
  [ -s "$file" ] || continue
  for where in staging production; do
    case $where in
      staging) url=${STATS_IMPORT_DATABASE_URL:-} ;;
      production) url=${PRODUCTION_STATS_IMPORT_DATABASE_URL:-} ;;
    esac
    [ -n "$url" ] || continue
    printf "%s into %s's: " "$(basename "$file")" "$where"
    DATABASE_URL=$url "$server" stats-import --totals-only "$file"
  done
done
