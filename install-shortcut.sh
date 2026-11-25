#!/usr/bin/env bash
# Installation de Koppy depuis une release OU depuis le dépôt local.
#
# Usage :
#   ./install-shortcut.sh                            # à la racine du dépôt : compile puis installe
#   KOPPY_BINARY=/chemin/koppy ./install-shortcut.sh  # depuis une archive de release
#
# Ce script ne télécharge rien : il installe le binaire fourni (ou compilé),
# l'icône, le lanceur et le raccourci clavier dans ~/.local. 
# Pour installer depuis une release GitHub, utilise tools/install.sh, qui fournit
# KOPPY_BINARY à ce script.
#
# Sous GNOME Wayland, une application ne peut pas capter elle-même des touches
# globales (modèle de sécurité Wayland). Le raccourci est donc déclaré comme
# raccourci personnalisé GNOME, qui exécute « koppy --activate ». L'instance
# déjà en cours s'affiche alors au premier plan, centrée. Si aucune instance
# ne tourne, l'application démarre normalement.

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${0}")" && pwd)"

# ---------------------------------------------------------------------------
# Choix du binaire à installer :
#   1. KOPPY_BINARY : transmis par tools/install.sh (archive de release) ;
#   2. ./koppy      : archive de release extraite manuellement ;
#   3. Cargo.toml   : dépôt local, compilation depuis les sources.
# ---------------------------------------------------------------------------
if [ -n "${KOPPY_BINARY:-}" ]; then
    SOURCE_BINARY="${KOPPY_BINARY}"
    if [ ! -x "${SOURCE_BINARY}" ]; then
        echo "erreur : binaire introuvable ou non exécutable : ${SOURCE_BINARY}." >&2
        exit 1
    fi
elif [ -x "${PROJECT_DIR}/koppy" ]; then
    SOURCE_BINARY="${PROJECT_DIR}/koppy"
elif [ -f "${PROJECT_DIR}/Cargo.toml" ]; then
    command -v cargo >/dev/null 2>&1 || {
        echo "erreur : cargo est requis pour compiler depuis les sources." >&2
        exit 1
    }
    echo '==> Compilation (release) depuis les sources locales…'
    cargo build --release --manifest-path "${PROJECT_DIR}/Cargo.toml"
    SOURCE_BINARY="${PROJECT_DIR}/target/release/koppy"
else
    echo "erreur : ni binaire (KOPPY_BINARY ou ./koppy) ni Cargo.toml dans ${PROJECT_DIR}." >&2
    echo "Ce script installe depuis une archive de release ou une copie locale du dépôt." >&2
    exit 1
fi

if [ ! -f "${PROJECT_DIR}/assets/koppy.svg" ]; then
    echo "erreur : icône introuvable : ${PROJECT_DIR}/assets/koppy.svg" >&2
    exit 1
fi

BIN_DIR="${HOME}/.local/bin"
BINARY="${BIN_DIR}/koppy"
ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
APP_DIR="${HOME}/.local/share/applications"

KEYBINDING_PATH='/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/koppy/'
SCHEMA='org.gnome.settings-daemon.plugins.media-keys.custom-keybinding'
MEDIA_KEYS='org.gnome.settings-daemon.plugins.media-keys'

echo "==> Installation du binaire dans ${BIN_DIR}…"
mkdir -p "${BIN_DIR}"
cp "${SOURCE_BINARY}" "${BINARY}"

echo '==> Installation de l’icône et du lanceur (barre des tâches)…'
mkdir -p "${ICON_DIR}" "${APP_DIR}"
cp "${PROJECT_DIR}/assets/koppy.svg" "${ICON_DIR}/koppy.svg"
cat > "${APP_DIR}/koppy.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Koppy — Presse-papier
GenericName=Gestionnaire du presse-papier
Comment=Historique du presse-papier avec épinglage
Exec=${BINARY} --activate
Icon=koppy
Terminal=false
Categories=Utility;
StartupWMClass=koppy
EOF
command -v gtk-update-icon-cache >/dev/null 2>&1 \
    && gtk-update-icon-cache -q -f -t "${HOME}/.local/share/icons/hicolor" || true
command -v update-desktop-database >/dev/null 2>&1 \
    && update-desktop-database -q "${APP_DIR}" || true

echo '==> Enregistrement du raccourci Shift+Super+V…'
if command -v gsettings >/dev/null 2>&1; then
    # « custom-keybindings » est une liste complète (remplacement, pas ajout).
    current="$(gsettings get "${MEDIA_KEYS}" custom-keybindings)"
    if [ "${current}" = '@as []' ] || [ -z "${current}" ]; then
        gsettings set "${MEDIA_KEYS}" custom-keybindings "['${KEYBINDING_PATH}']"
    else
        echo "Raccourcis personnalisés existants détectés : ${current}"
        echo "Ajoute manuellement '${KEYBINDING_PATH}' à cette liste, par exemple :"
        echo "  gsettings set ${MEDIA_KEYS} custom-keybindings \"${current%]}, '${KEYBINDING_PATH}']\""
    fi

    gsettings set "${SCHEMA}:${KEYBINDING_PATH}" name 'Koppy — Presse-papier'
    gsettings set "${SCHEMA}:${KEYBINDING_PATH}" command "${BINARY} --activate"
    gsettings set "${SCHEMA}:${KEYBINDING_PATH}" binding '<Shift><Super>v'
else
    echo "gsettings est absent : raccourci non enregistré."
fi

echo
echo 'Terminé ! Shift+Super+V ouvre maintenant Koppy au premier plan, centré.'
echo 'Tu peux aussi le vérifier/modifier dans Paramètres → Clavier → Raccourcis.'
