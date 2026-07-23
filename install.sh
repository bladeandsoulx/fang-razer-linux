#!/usr/bin/env bash
# Fang release installer.
#
# This script is safe to stream into Bash because every executable statement
# lives inside a function and the sole invocation is the final line.

fatal() {
  printf '%b✗%b %s\n' "${COLOR_ERROR:-}" "${COLOR_RESET:-}" "$*" >&2
  return 1
}

step() {
  printf '%b→%b %s\n' "${COLOR_CURRENT:-}" "${COLOR_RESET:-}" "$*"
}

complete() {
  printf '%b✓%b %s\n' "${COLOR_SUCCESS:-}" "${COLOR_RESET:-}" "$*"
}

warn() {
  printf '%b!%b %s\n' "${COLOR_WARNING:-}" "${COLOR_RESET:-}" "$*"
}

configure_output() {
  OUTPUT_TTY=0
  if [[ ${FANG_INSTALLER_TESTING:-} == 1 ]]; then
    OUTPUT_TTY=${FANG_TEST_TTY:-0}
  elif [[ -t 1 ]]; then
    OUTPUT_TTY=1
  fi

  COLOR_SUCCESS=
  COLOR_CURRENT=
  COLOR_WARNING=
  COLOR_ERROR=
  COLOR_BANNER=
  COLOR_RESET=
  if [[ $OUTPUT_TTY == 1 && -z ${NO_COLOR+x} ]]; then
    COLOR_SUCCESS=$'\033[32m'
    COLOR_CURRENT=$'\033[36m'
    COLOR_WARNING=$'\033[33m'
    COLOR_ERROR=$'\033[31m'
    COLOR_BANNER=$'\033[35m'
    COLOR_RESET=$'\033[0m'
  fi
}

print_banner() {
  [[ $OUTPUT_TTY == 1 ]] || return 0
  printf '%b%s%b\n' "$COLOR_BANNER" \
    '┌─────────────────────────────────────────────────────────┐' "$COLOR_RESET"
  printf '%b%s%b\n' "$COLOR_BANNER" \
    '│                  ◆ Fang Installer                       │' "$COLOR_RESET"
  printf '%b%s%b\n' "$COLOR_BANNER" \
    '├─────────────────────────────────────────────────────────┤' "$COLOR_RESET"
  printf '%b%s%b\n' "$COLOR_BANNER" \
    '│  Fan, power, lighting, and telemetry for Razer Blade.   │' "$COLOR_RESET"
  printf '%b%s%b\n' "$COLOR_BANNER" \
    '└─────────────────────────────────────────────────────────┘' "$COLOR_RESET"
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

  TARGET_USER=$(id -un)
  TARGET_UID=$(id -u)
  [[ -n $TARGET_USER && $TARGET_USER != root && $TARGET_UID =~ ^[0-9]+$ && $TARGET_UID != 0 ]] ||
    fatal 'Could not identify a non-root desktop user.'
  [[ $TARGET_USER =~ ^[a-z_][a-z0-9_-]*[$]?$ ]] ||
    fatal 'The desktop username contains unsupported characters.'
  passwd=$(getent passwd "$TARGET_USER") ||
    fatal "Could not resolve desktop user $TARGET_USER through getent."
  IFS=: read -r passwd_user _ passwd_uid _ _ TARGET_HOME _ <<< "$passwd"
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

parse_manifest() {
  local manifest=$1
  local line
  local digest
  local name
  local expected
  local line_count=0
  local expected_names=(
    install.sh
    "$DEB_FANG"
    "$DEB_FANGD"
    "$RPM_FANG"
    "$RPM_FANGD"
  )
  declare -A digests=()

  while IFS= read -r line; do
    ((line_count += 1))
    if [[ $line =~ ^([0-9a-f]{64})\ \ ([^/]+)$ ]]; then
      digest=${BASH_REMATCH[1]}
      name=${BASH_REMATCH[2]}
    else
      fatal "Malformed checksum manifest line $line_count."
    fi
    [[ -z ${digests[$name]+present} ]] ||
      fatal "Duplicate checksum manifest entry: $name"
    case $name in
      install.sh|"$DEB_FANG"|"$DEB_FANGD"|"$RPM_FANG"|"$RPM_FANGD") ;;
      *) fatal "Unexpected checksum manifest entry: $name" ;;
    esac
    digests[$name]=$digest
  done < "$manifest"

  [[ $line_count == 5 ]] ||
    fatal 'The checksum manifest must contain exactly five canonical entries.'
  for expected in "${expected_names[@]}"; do
    [[ -n ${digests[$expected]+present} ]] ||
      fatal "Missing checksum manifest entry: $expected"
  done

  {
    printf '%s  %s\n' "${digests[$SELECTED_FANG]}" "$SELECTED_FANG"
    printf '%s  %s\n' "${digests[$SELECTED_FANGD]}" "$SELECTED_FANGD"
  } > "$WORK_DIR/SELECTED_SHA256SUMS"
}

verify_checksums() {
  parse_manifest "$WORK_DIR/SHA256SUMS"
  if ! (
    cd "$WORK_DIR"
    sha256sum --check --strict SELECTED_SHA256SUMS
  ); then
    fatal 'Package checksum verification failed.'
  fi
}

verify_deb_field() {
  local file=$1
  local field=$2
  local expected=$3
  local actual

  actual=$(dpkg-deb -f "$file" "$field") ||
    fatal "Could not read DEB metadata field $field from ${file##*/}."
  [[ $actual == "$expected" ]] ||
    fatal "Invalid package metadata in ${file##*/}: $field is '$actual', expected '$expected'."
}

verify_deb_metadata() {
  local fang_file="$WORK_DIR/$DEB_FANG"
  local fangd_file="$WORK_DIR/$DEB_FANGD"

  verify_deb_field "$fang_file" Package fang
  verify_deb_field "$fang_file" Version "$DEB_FANG_VERSION"
  verify_deb_field "$fang_file" Architecture amd64
  verify_deb_field "$fangd_file" Package fangd
  verify_deb_field "$fangd_file" Version "$DEB_FANGD_VERSION"
  verify_deb_field "$fangd_file" Architecture amd64
}

verify_rpm_file() {
  local file=$1
  local expected_name=$2
  local output
  local fields=()

  output=$(rpm -qp --queryformat \
    $'%{NAME}\n%{EPOCH}\n%{VERSION}\n%{RELEASE}\n%{ARCH}\n' "$file") ||
    fatal "Could not read RPM package metadata from ${file##*/}."
  mapfile -t fields <<< "$output"
  [[ ${#fields[@]} == 5 ]] ||
    fatal "Invalid package metadata field count in ${file##*/}."
  [[ ${fields[0]} == "$expected_name" ]] ||
    fatal "Invalid package metadata in ${file##*/}: Name is '${fields[0]}'."
  [[ ${fields[1]} == '(none)' || ${fields[1]} == 0 ]] ||
    fatal "Invalid package metadata in ${file##*/}: Epoch is '${fields[1]}'."
  [[ ${fields[2]} == "$VERSION" ]] ||
    fatal "Invalid package metadata in ${file##*/}: Version is '${fields[2]}'."
  [[ ${fields[3]} == 1 ]] ||
    fatal "Invalid package metadata in ${file##*/}: Release is '${fields[3]}'."
  [[ ${fields[4]} == x86_64 ]] ||
    fatal "Invalid package metadata in ${file##*/}: Architecture is '${fields[4]}'."
}

verify_rpm_metadata() {
  verify_rpm_file "$WORK_DIR/$RPM_FANG" fang
  verify_rpm_file "$WORK_DIR/$RPM_FANGD" fangd
}

deb_state() {
  local package=$1
  local selected=$2
  local state_destination=$3
  local version_destination=$4
  local output
  local status
  local installed_version
  local state

  if ! output=$(dpkg-query -W -f='${Status}\t${Version}\n' "$package" 2>/dev/null); then
    printf -v "$state_destination" '%s' absent
    printf -v "$version_destination" '%s' '<absent>'
    return
  fi
  IFS=$'\t' read -r status installed_version <<< "$output"
  if [[ $status != 'install ok installed' ]]; then
    printf -v "$state_destination" '%s' absent
    printf -v "$version_destination" '%s' '<absent>'
    return
  fi
  [[ -n $installed_version ]] ||
    fatal "Installed DEB version is missing for $package."

  if dpkg --compare-versions "$installed_version" eq "$selected"; then
    state=equal
  elif dpkg --compare-versions "$installed_version" lt "$selected"; then
    state=older
  elif dpkg --compare-versions "$installed_version" gt "$selected"; then
    state=newer
  else
    fatal "Could not compare installed DEB version for $package."
  fi
  printf -v "$state_destination" '%s' "$state"
  printf -v "$version_destination" '%s' "$installed_version"
}

normalize_rpm_evr() {
  local evr=$1
  local destination=$2

  if [[ $evr == '(none):'* ]]; then
    evr="0:${evr#'(none):'}"
  fi
  [[ $evr =~ ^[0-9]+:[A-Za-z0-9._+~%-]+-[A-Za-z0-9._+~%-]+$ ]] ||
    fatal "Installed RPM returned an invalid EVR: $evr"
  printf -v "$destination" '%s' "$evr"
}

rpm_state() {
  local package=$1
  local selected=$2
  local state_destination=$3
  local version_destination=$4
  local output
  local installed_evr
  local comparison
  local state
  local records=()

  if ! output=$(rpm -q --queryformat $'%{EPOCH}:%{VERSION}-%{RELEASE}\n' "$package" 2>/dev/null); then
    printf -v "$state_destination" '%s' absent
    printf -v "$version_destination" '%s' '<absent>'
    return
  fi
  mapfile -t records <<< "$output"
  [[ ${#records[@]} == 1 ]] ||
    fatal "Installed RPM state is ambiguous for $package: ${#records[@]} records."
  normalize_rpm_evr "${records[0]}" installed_evr

  comparison=$(
    FANG_RPM_LEFT=$installed_evr FANG_RPM_RIGHT=$selected \
      rpm --eval '%{lua:print(rpm.vercmp(os.getenv("FANG_RPM_LEFT"), os.getenv("FANG_RPM_RIGHT")))}'
  ) || fatal "Could not compare installed RPM version for $package."
  [[ $comparison =~ ^-?[0-9]+$ ]] ||
    fatal "RPM returned an invalid version comparison for $package."
  if ((comparison == 0)); then
    state=equal
  elif ((comparison < 0)); then
    state=older
  else
    state=newer
  fi
  printf -v "$state_destination" '%s' "$state"
  printf -v "$version_destination" '%s' "$installed_evr"
}

decide_transaction() {
  if [[ $FANG_STATE == newer ]]; then
    fatal "Refusing downgrade: fang $FANG_INSTALLED is newer than selected $FANG_SELECTED_VERSION."
  fi
  if [[ $FANGD_STATE == newer ]]; then
    fatal "Refusing downgrade: fangd $FANGD_INSTALLED is newer than selected $FANGD_SELECTED_VERSION."
  fi
  if [[ $FANG_STATE == equal && $FANGD_STATE == equal ]]; then
    PACKAGE_TRANSACTION=0
  else
    PACKAGE_TRANSACTION=1
  fi
}

classify_installed_packages() {
  if [[ $PACKAGE_FAMILY == deb ]]; then
    FANG_SELECTED_VERSION=$DEB_FANG_VERSION
    FANGD_SELECTED_VERSION=$DEB_FANGD_VERSION
    deb_state fang "$FANG_SELECTED_VERSION" FANG_STATE FANG_INSTALLED
    deb_state fangd "$FANGD_SELECTED_VERSION" FANGD_STATE FANGD_INSTALLED
  else
    FANG_SELECTED_VERSION="0:${VERSION}-1"
    FANGD_SELECTED_VERSION="0:${VERSION}-1"
    rpm_state fang "$FANG_SELECTED_VERSION" FANG_STATE FANG_INSTALLED
    rpm_state fangd "$FANGD_SELECTED_VERSION" FANGD_STATE FANGD_INSTALLED
  fi
  decide_transaction
}

install_selected_packages() {
  if [[ $PACKAGE_TRANSACTION != 1 ]]; then
    return 0
  fi
  if [[ $PACKAGE_FAMILY == deb ]]; then
    sudo apt-get install -y "$WORK_DIR/$DEB_FANGD" "$WORK_DIR/$DEB_FANG"
  else
    sudo dnf install -y "$WORK_DIR/$RPM_FANGD" "$WORK_DIR/$RPM_FANG"
  fi
}

service_diagnostics() {
  systemctl status --no-pager --lines=20 fangd || true
}

reconcile_service() {
  if ! sudo systemctl enable --now fangd; then
    service_diagnostics
    fatal 'The fangd service could not be enabled. Run: sudo systemctl enable --now fangd'
  fi
  if ! systemctl is-active --quiet fangd; then
    service_diagnostics
    fatal 'The fangd service is not active. Run: sudo systemctl restart fangd'
  fi
  complete 'fangd is enabled and active'
}

confirm_group() {
  getent group fang >/dev/null ||
    fatal 'The fang group is missing after package installation.'
}

reconcile_group_membership() {
  local groups
  local group

  groups=$(id -nG "$TARGET_USER") ||
    fatal "Could not read group membership for $TARGET_USER."
  for group in $groups; do
    if [[ $group == fang ]]; then
      complete "$TARGET_USER is already in the fang group"
      return 0
    fi
  done
  if ! sudo usermod -aG fang "$TARGET_USER"; then
    fatal "Could not add $TARGET_USER to fang. Run: sudo usermod -aG fang $TARGET_USER"
  fi
  complete "Added $TARGET_USER to the fang group"
  warn 'Log out and back in once before launching Fang.'
}

mutate_system() {
  sudo -v
  install_selected_packages
  if [[ $PACKAGE_TRANSACTION == 1 ]]; then
    complete "Installed Fang $VERSION"
  else
    complete "Fang $VERSION packages are already installed"
  fi
  confirm_group
  reconcile_service
  reconcile_group_membership
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
readonly DEB_FANG_VERSION="$VERSION"
readonly DEB_FANGD_VERSION="${VERSION}-1"
readonly RPM_FANG="fang-${VERSION}-1.x86_64.rpm"
readonly RPM_FANGD="fangd-${VERSION}-1.x86_64.rpm"

  configure_output
  print_banner

  [[ $EUID != 0 ]] ||
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

  verify_checksums
  if [[ $PACKAGE_FAMILY == deb ]]; then
    verify_deb_metadata
  else
    verify_rpm_metadata
  fi
  classify_installed_packages
  complete 'Checksums and package metadata verified'
  mutate_system
}

main "$@"
