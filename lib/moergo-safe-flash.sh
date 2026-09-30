#!/usr/bin/env bash
# Shared Glove80/Go60 UF2 flashing and crash-loop recovery.

in_bootloader() { [ -f "$mnt/INFO_UF2.TXT" ] && [ -w "$mnt" ]; }

enter_bootloader() {
  in_bootloader && return 0
  local args=(bootloader --usb --yes) request_timeout=2
  [ -n "$peripheral" ] && request_timeout=20
  [ -n "$peripheral" ] && args+=(--peripheral)
  # A crash-looping central is only on USB ~1s per cycle; keep retrying
  # until a request lands in one of those windows.
  for _ in $(seq 1 60); do
    # The bootloader may mount after the initial check while a preceding
    # control attempt is still returning. Do not keep requesting an app that
    # has already rebooted.
    in_bootloader && return 0
    if ! read_device_report >/dev/null; then
      sleep 0.4
      continue
    fi
    if timeout "$request_timeout" "$CONTROL" "${args[@]}" >/dev/null 2>&1; then return 0; fi
    sleep 0.4
  done
  return 1
}

wait_mount() {
  # The auto-mounter races the volume's appearance; wait until the mount
  # is real and writable, then let it settle.
  for _ in $(seq 1 80); do
    in_bootloader && {
      sleep 1
      return 0
    }
    sleep 0.5
  done
  return 1
}

copy_image() {
  local img=$1
  cp "$img" "$mnt/" 2>/dev/null || {
    sleep 2
    cp "$img" "$mnt/" || return 1
  }
  sync
}

flash() {
  local img=$1
  enter_bootloader || {
    echo "FAIL: could not enter bootloader" >&2
    return 1
  }
  wait_mount || {
    echo "FAIL: bootloader volume never became writable" >&2
    return 1
  }
  copy_image "$img" || {
    echo "FAIL: could not copy UF2" >&2
    return 1
  }
  echo "flashed: $img"
}

# Query the application; USB presence alone cannot qualify a split peripheral.
read_device_report() {
  local report
  report=$(timeout 2 "$CONTROL" --usb battery --json 2>/dev/null) || return 1
  jq -e --arg board "$board_name" 'select(.name == $board)' <<<"$report"
}

target_present() {
  local report
  report=$(read_device_report) || return 1
  jq -e --arg board "$board_name" --arg peripheral "$peripheral" '
    .name == $board and
    ($peripheral == "" or any(.batteries[]; .name == "Peripheral 0" and .connected == true))
  ' <<<"$report" >/dev/null
}

verdict() {
  local deadline=$((SECONDS + watch + 40)) transitions=0 prev=x present first=""
  while [ "$SECONDS" -lt "$deadline" ]; do
    if target_present; then present=1; else present=0; fi
    if [ "$present" = 1 ] && [ -z "$first" ]; then
      first=$SECONDS
      deadline=$((SECONDS + watch))
    fi
    if [ "$prev" != x ] && [ "$present" != "$prev" ]; then
      transitions=$((transitions + 1))
      [ "$transitions" -ge 6 ] && {
        echo crashloop
        return
      }
    fi
    prev=$present
    sleep 0.3
  done
  if [ -n "$first" ] && [ "$present" = 1 ] && [ "$transitions" -le 1 ]; then
    echo stable
  else
    echo crashloop
  fi
}

moergo_flash_main() {
  set -uo pipefail
  local board=$1
  shift
  case "$board" in
  glove80)
    board_name=Glove80
    LH_VOL=GLV80LHBOOT
    RH_VOL=GLV80RHBOOT
    left_family=9807b007
    right_family=9808b007
    ;;
  go60)
    board_name=Go60
    LH_VOL=GO60LHBOOT
    RH_VOL=GO60RHBOOT
    left_family=9809b007
    right_family=980ab007
    ;;
  *)
    echo "unknown board: $board" >&2
    return 4
    ;;
  esac
  image="" recover="" peripheral="" watch=45
  while [ $# -gt 0 ]; do
    case "$1" in
    --recover | --watch-seconds)
      [ $# -ge 2 ] || {
        echo "$1 requires a value" >&2
        return 4
      }
      if [ "$1" = --recover ]; then recover=$2; else watch=$2; fi
      shift 2
      ;;
    --peripheral)
      peripheral=1
      shift
      ;;
    --no-watch)
      watch=0
      shift
      ;;
    -*)
      echo "unknown flag: $1" >&2
      return 4
      ;;
    *)
      [ -z "$image" ] || {
        echo "only one firmware image may be supplied" >&2
        return 4
      }
      image=$1
      shift
      ;;
    esac
  done
  [ -n "$image" ] && [ -f "$image" ] || {
    echo "usage: $board-safe-flash <image.uf2> [--recover <uf2>] [--peripheral] [--watch-seconds N] [--no-watch]" >&2
    return 4
  }
  [ -z "$recover" ] || [ -f "$recover" ] || {
    echo "recovery image not found: $recover" >&2
    return 4
  }
  [[ "$watch" =~ ^[0-9]{1,5}$ ]] || {
    echo "--watch-seconds must be an integer from 0 to 99999" >&2
    return 4
  }
  watch=$((10#$watch))

  local repo_root product_root script_path family build_output
  script_path=$(readlink -f "${BASH_SOURCE[0]}")
  repo_root=$(cd "$(dirname "$script_path")/.." && pwd)
  product_root="$repo_root/dependencies/moergo-rmk"
  if [ -z "${IN_NIX_SHELL:-}" ] && [ -z "${MOERGO_FLASH_DEV_SHELL:-}" ]; then
    local args=("$image" --watch-seconds "$watch")
    [ -z "$recover" ] || args+=(--recover "$recover")
    [ -z "$peripheral" ] || args+=(--peripheral)
    exec nix develop "path:$product_root" --command env MOERGO_FLASH_DEV_SHELL=1 bash "$script_path" "$board" "${args[@]}"
  fi

  # Cargo can relocate artifacts through target-dir, build-dir, or environment.
  build_output=$(cargo build --quiet --manifest-path "$product_root/Cargo.toml" \
    -p moergo-control -p xtask --message-format=json) || {
    echo "FAIL: could not build flash tools" >&2
    return 4
  }
  CONTROL=$(jq -ers '[.[] | select(.reason == "compiler-artifact" and .target.name == "moergo-control" and .executable != null) | .executable] | last // empty' <<<"$build_output") || return 4
  local xtask
  xtask=$(jq -ers '[.[] | select(.reason == "compiler-artifact" and .target.name == "xtask" and .executable != null) | .executable] | last // empty' <<<"$build_output") || return 4
  family=$left_family
  [ -z "$peripheral" ] || family=$right_family
  "$xtask" inspect-uf2 "$(readlink -f "$image")" --family "$family" || return 4
  if [ -n "$recover" ]; then
    "$xtask" inspect-uf2 "$(readlink -f "$recover")" --family "$family" || return 4
  fi
  local vol=$LH_VOL
  [ -z "$peripheral" ] || vol=$RH_VOL
  mnt=/run/media/$USER/$vol

  flash "$image" || return 4
  [ "$watch" = 0 ] && return 0
  echo "watching $board_name application health for ${watch}s..."
  local v
  v=$(verdict)
  echo "verdict: $v"
  [ "$v" = stable ] && return 0
  if [ -z "$recover" ]; then
    echo "application did not stay healthy and no --recover image given" >&2
    return 3
  fi
  echo "auto-recovering with $recover"
  flash "$recover" || return 3
  v=$(verdict)
  echo "recovery verdict: $v"
  [ "$v" = stable ] && return 2
  return 3
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  moergo_flash_main "$@"
  exit $?
fi
