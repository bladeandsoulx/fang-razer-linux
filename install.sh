#!/usr/bin/env bash
# Fang release installer.
#
# This script is safe to stream into Bash because every executable statement
# lives inside a function and the sole invocation is the final line.

fatal() {
  printf '✗ %s\n' "$*" >&2
  return 1
}

step() {
  printf '→ %s\n' "$*"
}

complete() {
  printf '✓ %s\n' "$*"
}

warn() {
  printf '! %s\n' "$*"
}

cleanup() {
  if [[ -n ${WORK_DIR:-} && -d ${WORK_DIR:-} ]]; then
    rm -rf -- "$WORK_DIR"
  fi
}

on_signal() {
  exit 130
}

decode_os_value() {
  local raw=$1
  local destination=$2
  local decoded=
  local inner
  local char
  local next
  local index

  if [[ ${raw:0:1} == '"' ]]; then
    [[ ${#raw} -ge 2 && ${raw: -1} == '"' ]] || return 1
    inner=${raw:1:${#raw}-2}
    index=0
    while ((index < ${#inner})); do
      char=${inner:index:1}
      if [[ $char == \\ ]]; then
        ((index += 1))
        ((index < ${#inner})) || return 1
        next=${inner:index:1}
        case $next in
          '$'|'`'|'"'|\\) decoded+=$next ;;
          *) return 1 ;;
        esac
      else
        decoded+=$char
      fi
      ((index += 1))
    done
  elif [[ ${raw:0:1} == "'" ]]; then
    [[ ${#raw} -ge 2 && ${raw: -1} == "'" ]] || return 1
    inner=${raw:1:${#raw}-2}
    [[ $inner != *"'"* ]] || return 1
    decoded=$inner
  else
    decoded=$raw
  fi
  printf -v "$destination" '%s' "$decoded"
}

valid_os_value() {
  local key=$1
  local value=$2
  case $key in
    ID) [[ $value =~ ^[a-z0-9._+-]+$ ]] ;;
    ID_LIKE) [[ $value =~ ^[a-z0-9._+-]+([[:space:]]+[a-z0-9._+-]+)*$ ]] ;;
    VERSION_ID) [[ $value =~ ^[A-Za-z0-9._+-]+$ ]] ;;
    VERSION_CODENAME|UBUNTU_CODENAME) [[ $value =~ ^[a-z0-9._+-]+$ ]] ;;
    PLATFORM_ID) [[ $value =~ ^[A-Za-z0-9:._+-]+$ ]] ;;
    *) return 1 ;;
  esac
}

parse_os_release() {
  local source=$1
  local line
  local key
  local raw
  local value
  declare -A seen=()

  OS_ID=
  OS_ID_LIKE=
  OS_VERSION_ID=
  OS_VERSION_CODENAME=
  OS_UBUNTU_CODENAME=
  OS_PLATFORM_ID=

  [[ -r $source ]] || fatal "Cannot read operating-system identity from $source."
  while IFS= read -r line || [[ -n $line ]]; do
    [[ -z $line || $line == \#* ]] && continue
    [[ $line == *=* ]] || fatal "Malformed os-release line: $line"
    key=${line%%=*}
    raw=${line#*=}
    case $key in
      ID|ID_LIKE|VERSION_ID|VERSION_CODENAME|UBUNTU_CODENAME|PLATFORM_ID) ;;
      *) continue ;;
    esac
    [[ -z ${seen[$key]+present} ]] || fatal "Duplicate os-release field: $key"
    seen[$key]=1
    value=
    decode_os_value "$raw" value || fatal "Malformed os-release value for $key."
    valid_os_value "$key" "$value" || fatal "Invalid os-release value for $key."
    case $key in
      ID) OS_ID=$value ;;
      ID_LIKE) OS_ID_LIKE=$value ;;
      VERSION_ID) OS_VERSION_ID=$value ;;
      VERSION_CODENAME) OS_VERSION_CODENAME=$value ;;
      UBUNTU_CODENAME) OS_UBUNTU_CODENAME=$value ;;
      PLATFORM_ID) OS_PLATFORM_ID=$value ;;
    esac
  done < "$source"
  [[ -n $OS_ID ]] || fatal 'Operating-system ID is missing.'
}

id_like_has() {
  local wanted=$1
  local item
  for item in $OS_ID_LIKE; do
    [[ $item == "$wanted" ]] && return 0
  done
  return 1
}

detect_platform() {
  local ubuntu_like=0
  local debian_like=0
  local fedora_like=0

  PACKAGE_FAMILY=
  PLATFORM_LABEL=
  DERIVATIVE_WARNING=

  case $OS_ID in
    ubuntu)
      case $OS_VERSION_ID:$OS_VERSION_CODENAME in
        22.04:jammy) PACKAGE_FAMILY=deb; PLATFORM_LABEL='Ubuntu 22.04' ;;
        24.04:noble) PACKAGE_FAMILY=deb; PLATFORM_LABEL='Ubuntu 24.04' ;;
        *) fatal "Unsupported Ubuntu release: ${OS_VERSION_ID:-unknown}." ;;
      esac
      return
      ;;
    debian)
      case $OS_VERSION_ID:$OS_VERSION_CODENAME in
        12:bookworm) PACKAGE_FAMILY=deb; PLATFORM_LABEL='Debian 12' ;;
        13:trixie) PACKAGE_FAMILY=deb; PLATFORM_LABEL='Debian 13' ;;
        *) fatal "Unsupported Debian release: ${OS_VERSION_ID:-unknown}." ;;
      esac
      return
      ;;
    fedora)
      case $OS_VERSION_ID:$OS_PLATFORM_ID in
        43:platform:f43) PACKAGE_FAMILY=rpm; PLATFORM_LABEL='Fedora 43' ;;
        44:platform:f44) PACKAGE_FAMILY=rpm; PLATFORM_LABEL='Fedora 44' ;;
        *) fatal "Unsupported Fedora release: ${OS_VERSION_ID:-unknown}." ;;
      esac
      return
      ;;
  esac

  id_like_has ubuntu && ubuntu_like=1
  id_like_has debian && debian_like=1
  id_like_has fedora && fedora_like=1
  if ((fedora_like && (ubuntu_like || debian_like))); then
    fatal "Conflicting distribution-family markers for $OS_ID."
  fi

  if ((ubuntu_like)); then
    case $OS_UBUNTU_CODENAME in
      jammy) PACKAGE_FAMILY=deb; PLATFORM_LABEL='Ubuntu 22.04' ;;
      noble) PACKAGE_FAMILY=deb; PLATFORM_LABEL='Ubuntu 24.04' ;;
      *) fatal "Unsupported or missing Ubuntu base for $OS_ID." ;;
    esac
  elif ((debian_like)); then
    case $OS_VERSION_CODENAME in
      bookworm) PACKAGE_FAMILY=deb; PLATFORM_LABEL='Debian 12' ;;
      trixie) PACKAGE_FAMILY=deb; PLATFORM_LABEL='Debian 13' ;;
      *) fatal "Unsupported or missing Debian base for $OS_ID." ;;
    esac
  elif ((fedora_like)); then
    case $OS_PLATFORM_ID in
      platform:f43) PACKAGE_FAMILY=rpm; PLATFORM_LABEL='Fedora 43' ;;
      platform:f44) PACKAGE_FAMILY=rpm; PLATFORM_LABEL='Fedora 44' ;;
      *) fatal "Unsupported or missing Fedora base for $OS_ID." ;;
    esac
  else
    fatal "Unsupported Linux distribution: $OS_ID."
  fi
  DERIVATIVE_WARNING="${OS_ID^} is compatible-family, not release-tested directly."
}

capture_identity() {
  local passwd
  local passwd_user
  local passwd_uid
  local ignored

  TARGET_USER=$(id -un)
  TARGET_UID=$(id -u)
  [[ -n $TARGET_USER && $TARGET_USER != root && $TARGET_UID =~ ^[0-9]+$ && $TARGET_UID != 0 ]] ||
    fatal 'Could not identify a non-root desktop user.'
  passwd=$(getent passwd "$TARGET_USER") ||
    fatal "Could not resolve desktop user $TARGET_USER through getent."
  IFS=: read -r passwd_user ignored passwd_uid ignored ignored TARGET_HOME ignored <<< "$passwd"
  [[ $passwd_user == "$TARGET_USER" && $passwd_uid == "$TARGET_UID" && $TARGET_HOME == /* ]] ||
    fatal "Invalid passwd entry for desktop user $TARGET_USER."
}

require_commands() {
  local command
  local commands=(curl sha256sum uname id getent mktemp systemctl sudo usermod)
  if [[ $PACKAGE_FAMILY == deb ]]; then
    commands+=(dpkg dpkg-deb dpkg-query apt-get)
  else
    commands+=(rpm dnf)
  fi
  for command in "${commands[@]}"; do
    command -v "$command" >/dev/null 2>&1 || fatal "Missing required command: $command"
  done
}

download_file() {
  local url=$1
  local destination=$2
  local partial="${destination}.part"
  rm -f -- "$partial"
  if ! curl --fail --show-error --silent --location \
    --proto '=https' --proto-redir '=https' \
    --retry 3 --retry-delay 1 \
    --output "$partial" "$url"; then
    rm -f -- "$partial"
    fatal "Download failed: ${url##*/}"
  fi
  mv -- "$partial" "$destination"
}

main() {
set -euo pipefail
umask 077
readonly VERSION='0.9.4'
readonly RELEASE_TAG='v0.9.4'
readonly REPOSITORY='bladeandsoulx/fang-razer-linux'
readonly RELEASE_BASE="https://github.com/${REPOSITORY}/releases/download/${RELEASE_TAG}"
readonly DEB_FANG="Fang_${VERSION}_amd64.deb"
readonly DEB_FANGD="fangd_${VERSION}-1_amd64.deb"
readonly RPM_FANG="fang-${VERSION}-1.x86_64.rpm"
readonly RPM_FANGD="fangd-${VERSION}-1.x86_64.rpm"

  local effective_euid=$EUID
  if [[ ${FANG_INSTALLER_TESTING:-} == 1 ]]; then
    effective_euid=${FANG_INSTALLER_TEST_EUID:-$effective_euid}
  fi
  [[ $effective_euid != 0 ]] ||
    fatal 'Run this installer as your desktop user without sudo.'
  [[ $(uname -m) == x86_64 ]] ||
    fatal 'Fang release packages support only x86_64 systems.'

  capture_identity
  parse_os_release "${FANG_OS_RELEASE_FILE:-/etc/os-release}"
  detect_platform
  require_commands

  complete "Detected: linux ($(
    if [[ -n $DERIVATIVE_WARNING ]]; then
      printf '%s → %s family' "$OS_ID" "$PLATFORM_LABEL"
    else
      printf '%s' "$PLATFORM_LABEL"
    fi
  ))"
  [[ -z $DERIVATIVE_WARNING ]] || warn "$DERIVATIVE_WARNING"

  WORK_DIR=$(mktemp -d "${TMPDIR:-/tmp}/fang-installer.XXXXXX")
  trap cleanup EXIT
  trap on_signal HUP INT TERM
  if [[ $PACKAGE_FAMILY == deb ]]; then
    SELECTED_FANG=$DEB_FANG
    SELECTED_FANGD=$DEB_FANGD
  else
    SELECTED_FANG=$RPM_FANG
    SELECTED_FANGD=$RPM_FANGD
  fi

  step "Downloading Fang $VERSION packages..."
  download_file "$RELEASE_BASE/SHA256SUMS" "$WORK_DIR/SHA256SUMS"
  download_file "$RELEASE_BASE/$SELECTED_FANG" "$WORK_DIR/$SELECTED_FANG"
  download_file "$RELEASE_BASE/$SELECTED_FANGD" "$WORK_DIR/$SELECTED_FANGD"
  complete "Downloaded Fang $VERSION package pair"
}

main "$@"
