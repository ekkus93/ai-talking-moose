#!/usr/bin/env bash
set -euo pipefail

# GitHub-hosted Ubuntu runners include third-party apt sources that are unrelated
# to this repository. A transiently inconsistent Chrome repository must not
# prevent us from refreshing the Ubuntu package indexes we actually depend on.
# Remove only source files that explicitly reference Chrome; retain apt's normal
# signature and hash verification for every remaining repository.
for source_file in /etc/apt/sources.list.d/*; do
  [[ -f "$source_file" ]] || continue
  if grep -qF 'dl.google.com/linux/chrome-stable/deb' "$source_file"; then
    echo "Disabling unrelated Chrome apt source: $source_file"
    sudo rm -f -- "$source_file"
  fi
done

sudo apt-get update
