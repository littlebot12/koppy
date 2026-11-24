#!/bin/sh
# Installateur « one-liner » pour Koppy.
#
#   sh -c "$(curl -fsSL https://raw.githubusercontent.com/littlebot12/koppy/main/tools/install.sh)"
#
# Télécharge la dernière release GitHub (binaire + icône + lanceur), vérifie
# l'empreinte SHA-256, puis délègue l'installation à install-shortcut.sh, fourni
# dans l'archive. Le script est volontairement compatible POSIX sh : il peut
# être exécuté directement via `curl … | sh` ou `sh -c "$(curl …)"`.
#
# Variables d'environnement facultatives :
#   KOPPY_VERSION=v0.1.0   installe un tag précis au lieu de la dernière release
#   KOPPY_SKIP_CHECKSUM=1  ignore la vérification SHA-256 (déconseillé)

set -eu

REPO='littlebot12/koppy'
ASSET='koppy-linux-x86_64.tar.gz'
BASE_URL="https://github.com/${REPO}/releases"

log() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
err() { printf '\033[1;31merreur :\033[0m %s\n' "$*" >&2; }

# ---------------------------------------------------------------------------
# Pré-requis
# ---------------------------------------------------------------------------
if [ "$(uname -s)" != 'Linux' ]; then
    err "Koppy n'est disponible que sous Linux pour l'instant."
    exit 1
fi

case "$(uname -m)" in
    x86_64|amd64) ;;
    *)
        err "Aucune release pour l'architecture $(uname -m) (x86_64 seulement)."
        err "Compile depuis les sources : https://github.com/${REPO}#compiler-depuis-les-sources"
        exit 1
        ;;
esac

if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
    err "curl ou wget est requis."
    exit 1
fi
command -v tar >/dev/null 2>&1 || { err "tar est requis."; exit 1; }

download() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --retry 3 -o "$2" "$1"
    else
        wget -q -O "$2" "$1"
    fi
}

# ---------------------------------------------------------------------------
# Téléchargement de la release
# ---------------------------------------------------------------------------
if [ -n "${KOPPY_VERSION:-}" ]; then
    RELEASE_URL="${BASE_URL}/download/${KOPPY_VERSION}"
else
    RELEASE_URL="${BASE_URL}/latest/download"
fi
ARCHIVE_URL="${RELEASE_URL}/${ASSET}"
CHECKSUM_URL="${ARCHIVE_URL}.sha256"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/koppy-install.XXXXXX")"
cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup 0
trap 'cleanup; exit 1' 1 2 15

log "Téléchargement de ${ASSET}…"
if ! download "$ARCHIVE_URL" "${TMP_DIR}/${ASSET}"; then
    err "Impossible de télécharger ${ARCHIVE_URL}."
    err "Vérifie qu'une release existe : https://github.com/${REPO}/releases"
    err "Sinon, récupère l'artefact depuis l'onglet Actions du dépôt."
    exit 1
fi

if [ "${KOPPY_SKIP_CHECKSUM:-0}" != '1' ]; then
    log "Vérification de l'empreinte SHA-256…"
    if download "$CHECKSUM_URL" "${TMP_DIR}/${ASSET}.sha256"; then
        (cd "$TMP_DIR" && sha256sum -c "${ASSET}.sha256") 2>/dev/null \
            || (cd "$TMP_DIR" && shasum -a 256 -c "${ASSET}.sha256") \
            || { err "Empreinte invalide : installation annulée."; exit 1; }
    else
        err "Fichier .sha256 introuvable : installation annulée."
        err "Définis KOPPY_SKIP_CHECKSUM=1 pour installer malgré tout."
        exit 1
    fi
fi

# ---------------------------------------------------------------------------
# Extraction et installation (binaire, icône, lanceur, raccourci GNOME)
# ---------------------------------------------------------------------------
log "Extraction…"
tar -xzf "${TMP_DIR}/${ASSET}" -C "$TMP_DIR"

if [ ! -x "${TMP_DIR}/koppy" ] \
    || [ ! -f "${TMP_DIR}/install-shortcut.sh" ] \
    || [ ! -f "${TMP_DIR}/assets/koppy.svg" ]; then
    err "Archive incomplète (binaire, script ou icône manquant)."
    exit 1
fi

log "Installation du binaire, de l'icône et du raccourci…"
chmod +x "${TMP_DIR}/install-shortcut.sh"
KOPPY_BINARY="${TMP_DIR}/koppy" "${TMP_DIR}/install-shortcut.sh"

log "Terminé."
printf 'Lance Koppy avec Shift+Super+V ou : %s/.local/bin/koppy\n' "$HOME"
