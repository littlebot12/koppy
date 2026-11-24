# Koppy

Gestionnaire du presse-papier pour Linux (GPUI / Rust) : historique automatique
de tout texte copié, épinglage, persistance, thème clair/sombre suivant le
système, et raccourci global **Shift + Super + V**.

## Page de présentation

Le site statique se trouve dans [`docs/`](docs/) et est publié sur
**[https://littlebot12.github.io/koppy/](https://littlebot12.github.io/koppy/)**.
Le déploiement est assuré par `.github/workflows/deploy-pages.yml`.

## Utilisation

- **Shift + Super + V** : affiche Koppy au premier plan, centré (démarre
  l'application si elle n'est pas déjà lancée). Sous Wayland, ce raccourci est
  enregistré dans les réglages GNOME et exécute `koppy --activate`.
- **Clic sur une carte** : recopie le texte dans le presse-papier puis ferme
  immédiatement la fenêtre, pour coller avec Ctrl+V sans étape supplémentaire.
- **Épingler / Supprimer** : actions sur chaque élément (un élément épinglé
  doit être désépinglé avant suppression).
- Les éléments épinglés survivent à « Tout supprimer » et aux redémarrages.
- Historique limité à 20 éléments, sauvegardé dans
  `~/.local/share/koppy/clipboard_history.json`.

### Ligne de commande

```bash
koppy            # lance l'application (ou active la fenêtre existante)
koppy --activate # active la fenêtre existante, ou la démarre si besoin
```

## Installation en une commande (Ubuntu 24.04, x86_64)

```bash
sh -c "$(curl -fsSL https://raw.githubusercontent.com/littlebot12/koppy/main/tools/install.sh)"
```

Le script `tools/install.sh` télécharge la dernière release, vérifie son
empreinte SHA-256, puis installe le binaire, l'icône, le lanceur et le raccourci
**Shift + Super + V**. Pour épingler une version précise :

```bash
KOPPY_VERSION=v0.1.0 sh -c "$(curl -fsSL https://raw.githubusercontent.com/littlebot12/koppy/main/tools/install.sh)"
```

## Installer l'archive de release manuellement

Télécharge `koppy-linux-x86_64.tar.gz` depuis les artefacts du workflow ou
depuis une release GitHub, puis :

```bash
mkdir koppy && tar -xzf koppy-linux-x86_64.tar.gz -C koppy
cd koppy
./install-shortcut.sh
```

Le script :

1. installe le binaire dans `~/.local/bin/koppy` ;
2. installe l'icône et le lanceur (`~/.local/share/applications/koppy.desktop`)
   pour que l'application apparaisse dans le dock avec sa propre icône ;
3. enregistre le raccourci **Shift + Super + V** dans GNOME.

Si tu as déjà des raccourcis personnalisés, le script t'indique comment ajouter
le chemin `koppy` à la liste au lieu de l'écraser.

## Compiler depuis les sources

Dépendances système (Ubuntu/Debian) :

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  pkg-config libxkbcommon-dev libxkbcommon-x11-dev \
  libxcb1-dev libxcb-xkb-dev libx11-dev libxrandr-dev libxi-dev \
  libwayland-dev libfontconfig1-dev libfreetype6-dev
```

Puis :

```bash
cargo build --release
./install-shortcut.sh   # depuis le dépôt : compile puis installe
```

## Intégration continue

`.github/workflows/build.yml` compile en release sur `ubuntu-24.04`, vérifie le
code avec Clippy (`-D warnings`) et produit une archive
`koppy-linux-x86_64.tar.gz` (avec son empreinte SHA-256) :

- à chaque push sur `main`, pull request ou déclenchement manuel : artefact
  téléchargeable depuis l'onglet **Actions** ;
- au push d'un tag `v*` : archive publiée dans une **Release** GitHub.

Le binaire étant lié à glibc 2.39 (Ubuntu 24.04), il faut une distribution
égale ou plus récente pour l'exécuter.
