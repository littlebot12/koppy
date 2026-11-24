use gpui::{div, prelude::*, px, svg, AnyElement, AssetSource, SharedString};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Permet à GPUI de charger les icônes SVG installées sur le système
/// (thème Adwaita / Yaru) — les chemins sont absolus.
pub struct SystemIcons;

impl AssetSource for SystemIcons {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        // Logos embarqués dans le binaire : GPUI ne fait que lire des octets
        // via AssetSource, il ne connaît pas le chemin du dépôt au runtime.
        match path {
            "koppy-symbolic" => {
                return Ok(Some(Cow::Borrowed(include_bytes!(
                    "../../assets/koppy-symbolic.svg"
                ))));
            }
            "koppy" => {
                return Ok(Some(Cow::Borrowed(include_bytes!("../../assets/koppy.svg"))));
            }
            _ => {}
        }

        match fs::read(path) {
            Ok(bytes) => Ok(Some(Cow::Owned(bytes))),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

/// Nom du thème d'icônes configuré (GNOME : Yaru, Adwaita, …), lu une seule
/// fois via gsettings ; `Adwaita` sert de repli universel.
fn system_icon_theme() -> Option<&'static str> {
    static THEME: OnceLock<Option<String>> = OnceLock::new();

    THEME
        .get_or_init(|| {
            let output = Command::new("gsettings")
                .args(["get", "org.gnome.desktop.interface", "icon-theme"])
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let name = String::from_utf8(output.stdout).ok()?;
            let name = name.trim().trim_matches('\'');
            (!name.is_empty()).then(|| name.to_string())
        })
        .as_deref()
}

/// Trouve une icône symbolique installée par le thème système.
fn system_icon(filename: &str) -> Option<SharedString> {
    let mut roots: Vec<PathBuf> = Vec::new();

    if let Some(theme) = system_icon_theme() {
        // Les variantes (ex. « Yaru-olive-dark ») n'ont pas toujours leur
        // propre dossier : on essaie aussi le thème de base (« Yaru »).
        let mut candidates = vec![theme];
        if let Some((base, _)) = theme.rsplit_once('-') {
            candidates.push(base);
        }

        for candidate in candidates {
            if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
                roots.push(PathBuf::from(&data_home).join("icons").join(candidate));
            } else if let Ok(home) = std::env::var("HOME") {
                roots.push(PathBuf::from(&home).join(".local/share/icons").join(candidate));
            }
            roots.push(PathBuf::from("/usr/share/icons").join(candidate));
            roots.push(PathBuf::from("/usr/local/share/icons").join(candidate));
        }
    }
    roots.push(PathBuf::from("/usr/share/icons/Adwaita"));

    for root in roots {
        // Les symboliques GTK se trouvent dans `symbolic/ui` (Adwaita) ou
        // `scalable/ui` (Yaru) ; on accepte aussi `actions` par compatibilité.
        for folder in ["symbolic/ui", "scalable/ui", "symbolic/actions", "scalable/actions"] {
            let path = root.join(folder).join(filename);
            if path.is_file() {
                return Some(path.to_string_lossy().into_owned().into());
            }
        }
    }
    None
}

/// Icône système teintée par GPUI avec la couleur du texte (donc clair/sombre),
/// ou caractère de repli si aucune icône n'est installée.
pub fn control_icon(kind: &str, fallback: &'static str, color: impl Into<gpui::Hsla>) -> AnyElement {
    action_icon(&format!("window-{kind}"), fallback, color)
}

/// Icône d'action générique (ex. `edit-delete`, `view-pin`).
/// La couleur doit être posée sur l'élément SVG lui-même : GPUI ne peint pas
/// un SVG dont le style n'a pas de `text.color` explicite (pas d'héritage).
pub fn action_icon(name: &str, fallback: &'static str, color: impl Into<gpui::Hsla>) -> AnyElement {
    let color = color.into();
    if let Some(path) = system_icon(&format!("{name}-symbolic.svg")) {
        svg()
            .path(path)
            .size(px(12.0))
            .text_color(color)
            .into_any_element()
    } else {
        div().child(fallback).text_color(color).into_any_element()
    }
}
