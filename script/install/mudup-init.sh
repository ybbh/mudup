#!/usr/bin/env sh
set -eu

TARGET="${MUDUP_INIT_TARGET:-x86_64-unknown-linux-gnu}"
INSTALL_BIN_DIR="${HOME}/.local/bin"
MUDUP_HOME_DIR="${MUDUP_HOME:-${HOME}/.mududb}"
MUDUP_PROXY_BIN_DIR="${MUDUP_HOME_DIR}/bin"
BASHRC="${HOME}/.bashrc"
PATH_LINE='export PATH="${HOME}/.local/bin:${HOME}/.mududb/bin:${PATH}"'

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command not found: $1" >&2
    exit 1
  fi
}

need_cmd curl
need_cmd tar
need_cmd sha256sum
need_cmd install
need_cmd mktemp
need_cmd find

detect_repo_from_git_origin() {
  if ! command -v git >/dev/null 2>&1; then
    return 1
  fi
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    return 1
  fi

  ORIGIN_URL="$(git config --get remote.origin.url 2>/dev/null || true)"
  if [ -z "${ORIGIN_URL}" ]; then
    return 1
  fi

  REPO_FROM_ORIGIN="$(
    printf '%s\n' "${ORIGIN_URL}" | sed -nE \
      -e 's#^https?://github\.com/([^/]+/[^/]+?)(\.git)?$#\1#p' \
      -e 's#^git@github\.com:([^/]+/[^/]+?)(\.git)?$#\1#p'
  )"
  if [ -n "${REPO_FROM_ORIGIN}" ]; then
    printf '%s\n' "${REPO_FROM_ORIGIN}"
    return 0
  fi
  return 1
}

normalize_repo() {
  # Accept either full GitHub URL or owner/repo and strip trailing .git.
  printf '%s\n' "$1" | sed -E \
    -e 's#^(git@github\.com:|https?://github\.com/)##' \
    -e 's#\.git$##' \
    -e 's#/$##'
}

REPO="$(normalize_repo "$(detect_repo_from_git_origin || true)")"
if [ -z "${REPO}" ]; then
  echo "error: cannot determine GitHub repository from git remote origin." >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT INT TERM

ARCHIVE="mudup-${TARGET}.tar.gz"
ARCHIVE_URL="https://github.com/${REPO}/releases/latest/download/${ARCHIVE}"
SHA256_URL="${ARCHIVE_URL}.sha256"

curl -fsSL "${ARCHIVE_URL}" -o "${TMP_DIR}/${ARCHIVE}"
curl -fsSL "${SHA256_URL}" -o "${TMP_DIR}/${ARCHIVE}.sha256"

(
  cd "${TMP_DIR}"
  CHECKSUM="$(awk 'NF { print $1; exit }' "${ARCHIVE}.sha256")"
  if [ -z "${CHECKSUM}" ]; then
    echo "error: empty checksum file ${ARCHIVE}.sha256" >&2
    exit 1
  fi
  printf '%s  %s\n' "${CHECKSUM}" "${ARCHIVE}" | sha256sum -c -
)

tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "${TMP_DIR}"
MUDUP_BIN_PATH="$(find "${TMP_DIR}" -type f -path '*/bin/mudup' | head -n 1)"
if [ -z "${MUDUP_BIN_PATH}" ]; then
  echo "error: mudup binary not found in archive ${ARCHIVE}" >&2
  exit 1
fi

mkdir -p "${INSTALL_BIN_DIR}"
install -m 0755 "${MUDUP_BIN_PATH}" "${INSTALL_BIN_DIR}/mudup"

if [ -f "${BASHRC}" ]; then
  if ! grep -qxF "${PATH_LINE}" "${BASHRC}"; then
    printf '%s\n' "${PATH_LINE}" >> "${BASHRC}"
  fi
else
  printf '%s\n' "${PATH_LINE}" > "${BASHRC}"
fi

export PATH="${HOME}/.local/bin:${MUDUP_PROXY_BIN_DIR}:${PATH}"

echo "mudup installed: ${INSTALL_BIN_DIR}/mudup"
echo "PATH updated for current shell and persisted in ${BASHRC}"
echo "next: mudup install"
