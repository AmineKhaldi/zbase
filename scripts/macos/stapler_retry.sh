#!/usr/bin/env bash
# Apple's ticket service can drop connections even after notarization succeeds.
# Always require a successful staple/validation; only retry service failures.
set -euo pipefail
: "${1:?Usage: stapler_retry.sh staple|validate path}"
: "${2:?Usage: stapler_retry.sh staple|validate path}"
attempt=1
delay=10
while true; do
  if xcrun stapler "$@"; then
    exit 0
  else
    status=$?
  fi
  case "${status}" in
    68|69|75) ;; # EX_NOHOST, EX_UNAVAILABLE, EX_TEMPFAIL
    *) exit "${status}" ;;
  esac
  if (( attempt >= 4 )); then
    echo "stapler $1 failed after ${attempt} attempts (exit ${status})" >&2
    exit "${status}"
  fi
  echo "stapler $1 service failure (exit ${status}); retrying in ${delay}s" >&2
  sleep "${delay}"
  attempt=$((attempt + 1))
  delay=$((delay * 2))
done
