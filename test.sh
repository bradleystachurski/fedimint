#!/usr/bin/env bash

fed_version=v0.5.0
client_version=v0.4.4
gateway_version=v0.6.0-rc.1

export LNV2_STABLE_VERSION="v0.6-rc.1"
function version_lt() {
  #[ "$1" != "$(echo -e "$1\n$2" | sort -V | tail -n 1)" ]
  if [ "$1" = "current" ] && [ "$2" = "current" ]; then
    return 1
  elif [ "$1" = "current" ]; then
    return 1
  elif [ "$2" = "current" ]; then
    return 0
  else
    echo "inside version_lt for echo comparison"
    first="$(echo -e "$1\n$2")"
    echo "first:"
    echo "$first"
    echo ""
    second="$(echo -e "$1\n$2" | sort -V)"
    echo "second:"
    echo "$second"
    echo ""
    third="$(echo -e "$1\n$2" | sort -V | tail -n 1)"
    echo "third:"
    echo "$third"
    echo ""
    res="$(echo -e "$1\n$2" | sort -V | tail -n 1)"
    echo "res: $res"
    [ "$1" != "$(echo -e "$1\n$2" | sort -V | tail -n 1)" ]
  fi
}

function supports_lnv2() {
  fed_version=$1
  client_version=$2
  gateway_version=$3

  if version_lt $fed_version $LNV2_STABLE_VERSION; then
      return 1
  fi

  if version_lt $client_version $LNV2_STABLE_VERSION; then
      return 1
  fi

  if version_lt $gateway_version $LNV2_STABLE_VERSION; then
      return 1
  fi

  return 0
}

if supports_lnv2 "$fed_version" "$client_version" "$gateway_version"; then
  echo "supports lnv2"
fi

function version_lt() {
  if [ "$1" = "current" ] && [ "$2" = "current" ]; then
    return 1
  elif [ "$1" = "current" ]; then
    return 1
  elif [ "$2" = "current" ]; then
    return 0
  else
    [ "$1" != "$(echo -e "$1\n$2" | sort -V | tail -n 1)" ]
  fi
}
