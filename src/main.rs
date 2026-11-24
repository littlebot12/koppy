// Le dossier s'appelle `lib`, mais `mod lib` déclenche le lint
// `special_module_name` (réservé à la racine de la target library).
#[path = "lib/mod.rs"]
mod app;

use app::{assets::SystemIcons, clipboard::ClipboardManager};
use async_channel::Sender;
use gpui::{
    px, size, AnyWindowHandle, App, AppContext, Application, Entity, TitlebarOptions, WindowBounds,
    WindowDecorations, WindowOptions,
};
use std::path::PathBuf;
use std::time::Duration;

const APP_TITLE: &str = "Koppy — Presse-papier";

fn default_window_size() -> gpui::Size<gpui::Pixels> {
    size(px(760.0), px(620.0))
}
/// Ouvre la fenêtre principale (utilisé au démarrage et à chaque activation).
fn open_main_window(
    cx: &mut App,
    entity: &Entity<ClipboardManager>,
    window_size: gpui::Size<gpui::Pixels>,
) -> AnyWindowHandle {
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(window_size, cx)),
                kind: gpui::WindowKind::Floating,
                titlebar: Some(TitlebarOptions {
                    title: Some("Koppy".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                // Sous Wayland, GNOME n'applique pas les décorations côté
                // serveur aux fenêtres natives : on dessine notre propre barre
                // avec les icônes SVG du thème système.
                window_decorations: Some(WindowDecorations::Client),
                focus: true,
                window_min_size: Some(size(px(420.0), px(520.0))),
                app_id: Some("koppy".to_string()),
                ..Default::default()
            },
            |window, _cx| {
                window.set_window_title(APP_TITLE);

                window
                    .observe_window_appearance(|window, _| window.refresh())
                    .detach();

                entity.clone()
            },
        )
        .expect("impossible d’ouvrir la fenêtre Koppy");

    handle.into()
}

/// Ré-affiche la fenêtre au premier plan, centrée sur l'écran, et retourne
/// le handle de la nouvelle fenêtre.
fn reopen_window_centered(
    cx: &mut App,
    entity: &Entity<ClipboardManager>,
    previous_handle: Option<AnyWindowHandle>,
) -> AnyWindowHandle {
    let (current_size, maximized) = previous_handle
        .and_then(|handle| {
            handle
                .update(cx, |_, window, _| {
                    (
                        window.bounds().size,
                        matches!(window.window_bounds(), WindowBounds::Maximized(_)),
                    )
                })
                .ok()
        })
        .unwrap_or((default_window_size(), false));

    let window_size =
        if maximized || current_size.width <= px(0.0) || current_size.height <= px(0.0) {
            default_window_size()
        } else {
            current_size
        };

    if let Some(handle) = previous_handle {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }

    let handle = open_main_window(cx, entity, window_size);
    // Demande aussi l'activation côté compositeur (Wayland/X11).
    let _ = handle.update(cx, |_, window, _| window.activate_window());
    handle
}

fn main() {
    if send_activation() {
        return;
    }

    Application::new().with_assets(SystemIcons).run(|cx: &mut App| {
        let (tx, rx) = async_channel::unbounded::<()>();
        let entity = cx.new(ClipboardManager::new);

        listen_for_activation(tx.clone());
        grab_global_hotkey(tx.clone());

        let activation_entity = entity.clone();
        cx.spawn(async move |cx| {
            for _ in 0..50 {
                let has_display = cx.update(|cx| !cx.displays().is_empty()).unwrap_or(false);
                if has_display {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(20))
                    .await;
            }

            let mut window_handle = cx
                .update(|cx| {
                    Some(open_main_window(
                        cx,
                        &activation_entity,
                        default_window_size(),
                    ))
                })
                .ok()
                .flatten();

            while rx.recv().await.is_ok() {
                let entity = activation_entity.clone();
                let previous_handle = window_handle;

                window_handle = cx
                    .update(move |cx| Some(reopen_window_centered(cx, &entity, previous_handle)))
                    .ok()
                    .flatten();

                if window_handle.is_none() {
                    break; // L'application est en train de se fermer.
                }
            }
        })
        .detach();
    });
}


#[cfg(unix)]
fn activation_socket_path() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("TMPDIR").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("koppy.sock")
}

#[cfg(unix)]
fn send_activation() -> bool {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    let Ok(mut stream) = UnixStream::connect(activation_socket_path()) else {
        return false;
    };

    stream.write_all(b"activate").is_ok() && stream.flush().is_ok()
}

#[cfg(not(unix))]
fn send_activation() -> bool {
    false
}

/// Écoute la socket d'activation. Chaque message reçu déclenche
/// l'affichage de la fenêtre via le canal `tx`.
#[cfg(unix)]
fn listen_for_activation(tx: Sender<()>) {
    use std::io::Read;
    use std::os::unix::net::UnixListener;

    let path = activation_socket_path();
    let listener = match UnixListener::bind(&path) {
        Ok(listener) => listener,
        // Socket orpheline (instance tuée sans nettoyage) : on la retire.
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
            let _ = std::fs::remove_file(&path);
            match UnixListener::bind(&path) {
                Ok(listener) => listener,
                Err(err) => {
                    eprintln!("koppy : socket d’activation indisponible ({err})");
                    return;
                }
            }
        }
        Err(err) => {
            eprintln!("koppy : socket d’activation indisponible ({err})");
            return;
        }
    };

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            let mut buffer = [0u8; 16];
            if stream.read(&mut buffer).is_ok() && buffer.starts_with(b"activate") {
                let _ = tx.send_blocking(());
            }
        }
    });
}

#[cfg(not(unix))]
fn listen_for_activation(_tx: Sender<()>) {}

/// Enregistre Shift+Super+V comme raccourci global X11 et notifie `tx` à
/// chaque pression. Sans effet sous Wayland GNOME (les grabs X11 y sont
/// bloqués) : l'activation passe alors par le raccourci système GNOME,
/// configuré pour exécuter `koppy --activate`.
#[cfg(target_os = "linux")]
fn grab_global_hotkey(tx: Sender<()>) {
    std::thread::spawn(move || {
        use x11rb::connection::Connection as _;
        use x11rb::protocol::xproto::{ConnectionExt as _, GrabMode, ModMask};

        const V_KEYSYM: u32 = 0x76; // touche « v »
        let shift_super = ModMask::SHIFT | ModMask::M4;

        // Connexion à X11 (XWayland sous une session Wayland).
        let Ok((conn, screen)) = x11rb::rust_connection::RustConnection::connect(None) else {
            return;
        };
        let root = conn.setup().roots[screen].root;

        // Recherche du keycode correspondant à la touche « V ».
        let setup = conn.setup();
        let min_keycode = setup.min_keycode;
        let keycode_count =
            u8::try_from(u32::from(setup.max_keycode) - u32::from(min_keycode) + 1).unwrap_or(0);
        let Ok(cookie) = conn.get_keyboard_mapping(min_keycode, keycode_count) else {
            return;
        };
        let Ok(mapping) = cookie.reply() else {
            return;
        };
        let keysyms_per_keycode = usize::from(mapping.keysyms_per_keycode);
        let Some(keycode) = mapping
            .keysyms
            .chunks_exact(keysyms_per_keycode)
            .position(|chunk| chunk.contains(&V_KEYSYM))
            .map(|index| min_keycode + index as u8)
        else {
            return;
        };

        // Variantes avec CapsLock / NumLock actifs, sinon le raccourci ne se
        // déclencherait pas quand ces verrous sont allumés.
        let lock = ModMask::LOCK;
        let numlock = ModMask::M2;
        let variants = [
            shift_super,
            shift_super | lock,
            shift_super | numlock,
            shift_super | lock | numlock,
        ];
        let grabs_ok = variants.iter().all(|modifiers| {
            conn.grab_key(
                false,
                root,
                *modifiers,
                keycode,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
            )
            .is_ok()
        });
        if !grabs_ok || conn.flush().is_err() {
            eprintln!("koppy : raccourci global X11 refusé par le gestionnaire de fenêtres");
            return;
        }

        loop {
            match conn.wait_for_event() {
                Ok(x11rb::protocol::Event::KeyPress(_)) => {
                    let _ = tx.send_blocking(());
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
}

#[cfg(not(target_os = "linux"))]
fn grab_global_hotkey(_tx: Sender<()>) {}
