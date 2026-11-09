#!/usr/bin/env bash
# Waits until a store serves the extension's package for each system given.
#
# Both stores answer "Published" before a new package can be installed: they
# check it first, for some minutes, and the Marketplace has dropped packages
# uploaded during that check without saying so. This fails the job instead.
#
# Usage: await-extension.sh marketplace|open-vsx <package.vsix>...
# where each package is named ortho-vscode-<system>.vsix, as the vscode job names
# them, and the version is editors/vscode/package.json's.
set -euo pipefail

store=$1
shift
manifest=editors/vscode/package.json
publisher=$(jq -r .publisher "$manifest")
name=$(jq -r .name "$manifest")
version=$(jq -r .version "$manifest")
deadline=$((SECONDS + 30 * 60))

for vsix in "$@"; do
  target=$(basename "$vsix" .vsix)
  target=${target#ortho-vscode-}
  case $store in
    marketplace) url="https://marketplace.visualstudio.com/_apis/public/gallery/publishers/$publisher/vsextensions/$name/$version/vspackage?targetPlatform=$target" ;;
    open-vsx) url="https://open-vsx.org/api/$publisher/$name/$target/$version" ;;
    *) echo "::error::Unknown store $store" && exit 2 ;;
  esac
  until curl -sfo /dev/null -A "$name-release" "$url"; do
    if ((SECONDS > deadline)); then
      echo "::error::$store does not serve $publisher.$name $version for $target after 30 minutes. Run this job again: --skip-duplicate publishes only what is missing."
      exit 1
    fi
    sleep 30
  done
  echo "$store serves $publisher.$name $version for $target"
done
