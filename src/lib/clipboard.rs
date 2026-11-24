use crate::app::assets::{action_icon, control_icon};
use crate::app::theme::Theme;
use arboard::Clipboard;
use dirs::data_local_dir;
use gpui::{
    div, prelude::*, px, relative, rgb, svg, Context, Decorations, FontWeight,
    InteractiveElement, Render, StatefulInteractiveElement, Styled, Window,
    WindowControlArea,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const MAX_ITEMS: usize = 30;

const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardItem {
    id: u64,
    text: String,
    pinned: bool,
}

#[derive(Default)]
pub struct ClipboardManager {
    items: Vec<ClipboardItem>,
    next_id: u64,
}

impl ClipboardManager {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut manager = Self::default();
        manager.load();

        cx.spawn(async move |this, cx| {
            loop {
                // La lecture peut bloquer : on la fait hors du thread UI.
                let text = cx
                    .background_spawn(async move {
                        Clipboard::new()
                            .ok()
                            .and_then(|mut clipboard| clipboard.get_text().ok())
                    })
                    .await;

                let entity_alive = this
                    .update(cx, |state, cx| {
                        if let Some(text) = text {
                            state.record_copied_text(text, cx);
                        }
                    })
                    .is_ok();

                if !entity_alive {
                    break;
                }

                cx.background_executor().timer(POLL_INTERVAL).await;
            }
        })
        .detach();

        manager
    }

    fn storage_path() -> PathBuf {
        let base_dir = data_local_dir().unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local/share")
        });

        base_dir.join("koppy").join("clipboard_history.json")
    }

    fn save(&self) {
        if let Some(parent) = Self::storage_path().parent() {
            let _ = fs::create_dir_all(parent);
        }

        let payload =
            serde_json::to_string_pretty(&self.items).unwrap_or_else(|_| "[]".to_string());
        let _ = fs::write(Self::storage_path(), payload);
    }

    fn load(&mut self) {
        let path = Self::storage_path();
        let Ok(contents) = fs::read_to_string(path) else {
            return;
        };

        let Ok(items) = serde_json::from_str::<Vec<ClipboardItem>>(&contents) else {
            return;
        };

        self.items = items;
        self.next_id = self.items.iter().map(|item| item.id).max().unwrap_or(0) + 1;
    }

    /// Aperçu d'un élément : 3 lignes maximum, alignées sur la hauteur fixe
    /// des cartes (les lignes surnuméraires sont rognées par `overflow_hidden`).
    fn preview_text(text: &str) -> String {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let compact = normalized.lines().take(3).collect::<Vec<_>>().join("\n");

        if compact.chars().count() > 140 {
            let truncated: String = compact.chars().take(137).collect();
            format!("{truncated}…")
        } else {
            compact
        }
    }

    /// Enregistre un texte observé dans le presse-papier système.
    /// S'il est déjà présent dans l'historique, on le remonte simplement en
    /// tête de liste (comportement Win+V) au lieu de créer un doublon.
    fn record_copied_text(&mut self, text: String, cx: &mut Context<Self>) {
        let clean = text.trim().to_string();
        if clean.is_empty() {
            return;
        }

        if let Some(index) = self.items.iter().position(|item| item.text == clean) {
            if index == 0 {
                return;
            }
            let item = self.items.remove(index);
            self.items.insert(0, item);
            self.save();
            cx.notify();
            return;
        }

        self.add_text(clean);
        cx.notify();
    }

    fn add_text(&mut self, text: String) {
        let clean = text.trim().to_string();
        if clean.is_empty() {
            return;
        }

        self.items.insert(
            0,
            ClipboardItem {
                id: self.next_id,
                text: clean,
                pinned: false,
            },
        );
        self.next_id += 1;

        while self.items.len() > MAX_ITEMS {
            let remove_index = self
                .items
                .iter()
                .position(|item| !item.pinned)
                .unwrap_or(self.items.len() - 1);
            self.items.remove(remove_index);
        }

        self.save();
    }

    fn toggle_pin(&mut self, id: u64) {
        if let Some(item) = self.items.iter_mut().find(|entry| entry.id == id) {
            item.pinned = !item.pinned;
            self.save();
        }
    }

    fn remove_item(&mut self, id: u64) {
        let Some(item) = self.items.iter().find(|entry| entry.id == id).cloned() else {
            return;
        };

        if item.pinned {
            return;
        }

        self.items.retain(|entry| entry.id != id);
        self.save();
    }

    fn clear_unpinned(&mut self) {
        self.items.retain(|item| item.pinned);
        self.save();
    }

    fn copy_to_system(&self, text: &str) {
        let Ok(mut clipboard) = Clipboard::new() else {
            return;
        };

        let _ = clipboard.set_text(text.to_string());
    }

    /// Barre de titre dessinée par l'app : Wayland n'applique pas les décorations côté serveur.
    fn render_titlebar(&self, theme: &Theme, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let icon_color = if theme.dark {
            rgb(0xFFFFFF)
        } else {
            rgb(0x000000)
        };

        let minimize_button = div()
            .id("window-minimize")
            .size(px(30.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .text_color(icon_color)
            .hover(|style| style.bg(rgb(theme.btn_hover)).text_color(rgb(theme.text)))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|_, _, window, _| {
                    window.minimize_window();
                }),
            )
            .child(control_icon("minimize", "–", icon_color));

        let maximize_button = div()
            .id("window-maximize")
            .size(px(30.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .text_color(icon_color)
            .hover(|style| style.bg(rgb(theme.btn_hover)).text_color(rgb(theme.text)))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|_, _, window, _| {
                    window.zoom_window();
                }),
            )
            .child(control_icon(
                if window.is_maximized() {
                    "restore"
                } else {
                    "maximize"
                },
                if window.is_maximized() { "❐" } else { "□" },
                icon_color,
            ));

        let close_button = div()
            .id("window-close")
            .size(px(30.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .text_color(icon_color)
            // Seule exception au monochrome : le rouge reste lisible dans les deux thèmes.
            .hover(|style| {
                style.bg(rgb(if theme.dark { 0xC62828 } else { 0xFFCDD2 }))
            })
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|_, _, window, _| {
                    window.remove_window();
                }),
            )
            .child(control_icon("close", "×", icon_color));

        div()
            .id("titlebar")
            .flex()
            .items_center()
            .gap_2()
            .h(px(40.0))
            .pl_4()
            .pr_2()
            .bg(rgb(theme.titlebar))
            .border_b_1()
            .border_color(rgb(theme.border))
            .child(
                svg()
                    .path("koppy-symbolic")
                    .size(px(22.0))
                    .text_color(icon_color),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(theme.text))
                    .child("Koppy"),
            )
            .child(
                div()
                    .id("titlebar-drag")
                    .flex_1()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag)
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|_, _, window, _| {
                            window.start_window_move();
                        }),
                    ),
            )
            .child(minimize_button)
            .child(maximize_button)
            .child(close_button)
    }
}

impl Render for ClipboardManager {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_custom_titlebar = matches!(window.window_decorations(), Decorations::Client { .. });

        let theme = Theme::from_appearance(window.appearance());

        let total = self.items.len();
        let pinned_count = self.items.iter().filter(|entry| entry.pinned).count();

        let mut items = self.items.clone();
        items.sort_by_key(|entry| !entry.pinned);

        let empty_state = div()
            .id("empty-state")
            .flex()
            .flex_1()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .text_color(rgb(theme.muted))
            .child(div().text_sm().child("Aucun élément dans l’historique."))
            .child(
                div()
                    .text_xs()
                    .child("Copiez du texte n’importe où, il apparaîtra ici automatiquement."),
            )
            .into_any_element();

        let grid = div()
            .id("clipboard-grid")
            .grid()
            .grid_cols(2)
            .items_start()
            .gap_4()
            .pr_1()
            .overflow_y_scroll()
            .children(items.into_iter().map(|entry| {
                let entry_id = entry.id;
                let entry_text = entry.text.clone();
                let pinned = entry.pinned;
                let preview = Self::preview_text(&entry.text);
                let pin_label = if pinned { "Désépingler" } else { "Épingler" };

                div()
                    .id(("card", entry_id))
                    .rounded_lg()
                    .bg(if pinned {
                        rgb(theme.card_pinned)
                    } else {
                        rgb(theme.card)
                    })
                    .when(!pinned, |card| {
                        card.hover(|style| style.bg(rgb(theme.card_hover)))
                    })
                    .border_1()
                    .border_color(if pinned {
                        rgb(theme.border_pinned)
                    } else {
                        rgb(theme.border)
                    })
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    // Hauteur fixe : en-tête + 3 lignes de texte.
                    .h(px(124.0))
                    .overflow_hidden()
                    // Copie puis fermeture immédiate : comportement Win+V.
                    .on_click(cx.listener(move |this, _, window, _| {
                        this.copy_to_system(&entry_text);
                        window.remove_window();
                    }))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if pinned {
                                        rgb(theme.text)
                                    } else {
                                        rgb(theme.muted)
                                    })
                                    .child(if pinned { "Épinglé" } else { "Historique" }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .id(("pin", entry_id))
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .text_xs()
                                            .bg(if pinned {
                                                rgb(theme.solid)
                                            } else {
                                                rgb(theme.btn)
                                            })
                                            .text_color(if pinned {
                                                rgb(theme.on_solid)
                                            } else {
                                                rgb(theme.text)
                                            })
                                            .hover(|style| {
                                                style.bg(if pinned {
                                                    rgb(if theme.dark { 0xD4D4D4 } else { 0x525252 })
                                                } else {
                                                    rgb(theme.btn_hover)
                                                })
                                            })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                this.toggle_pin(entry_id);
                                                cx.notify();
                                            }))
                                            .child(action_icon(
                                                "view-pin",
                                                "📌",
                                                if pinned {
                                                    rgb(theme.on_solid)
                                                } else {
                                                    rgb(theme.text)
                                                },
                                            ))
                                            .child(pin_label),
                                    )
                                    .when(!pinned, |row| {
                                        row.child(
                                            div()
                                                .id(("remove", entry_id))
                                                .px_2()
                                                .py_1()
                                                .rounded_md()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .text_xs()
                                                .bg(rgb(theme.btn))
                                                .text_color(rgb(theme.text))
                                                .hover(|style| {
                                                    style.bg(rgb(if theme.dark {
                                                        0xC62828
                                                    } else {
                                                        0xFFCDD2
                                                    }))
                                                })
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    cx.stop_propagation();
                                                    this.remove_item(entry_id);
                                                    cx.notify();
                                                }))
                                                .child(action_icon(
                                                    "edit-delete",
                                                    "🗑",
                                                    rgb(theme.text),
                                                ))
                                                .child("Supprimer"),
                                        )
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .text_sm()
                            .line_height(relative(1.4))
                            .whitespace_normal()
                            .text_color(rgb(theme.text))
                            .child(preview),
                    )
            }))
            .into_any_element();

        let content = if total == 0 { empty_state } else { grid };

        let header = div()
            .flex()
            .justify_between()
            .items_center()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(theme.text))
                            .child("Presse-papier"),
                    )
                    .child(div().text_xs().text_color(rgb(theme.muted)).child(format!(
                        "{} élément{} · {} épinglé{}",
                        total,
                        if total > 1 { "s" } else { "" },
                        pinned_count,
                        if pinned_count > 1 { "s" } else { "" },
                    ))),
            )
            .child(
                div()
                    .id("clear-all")
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_sm()
                    .border_1()
                    .border_color(rgb(theme.border))
                    .text_color(rgb(theme.muted))
                    .hover(|style| {
                        style
                            .bg(rgb(theme.card_hover))
                            .border_color(rgb(theme.border_pinned))
                            .text_color(rgb(theme.text))
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.clear_unpinned();
                        cx.notify();
                    }))
                    .child(action_icon("edit-clear-all", "⌫", rgb(theme.muted)))
                    .child("Tout supprimer"),
            );

        div()
            .id("root")
            .bg(rgb(theme.bg))
            .text_color(rgb(theme.text))
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .when(has_custom_titlebar, |root| {
                root.child(self.render_titlebar(&theme, window, cx))
            })
            .child(
                div()
                    .id("content")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_4()
                    .p_4()
                    .overflow_hidden()
                    .child(header)
                    .child(content),
            )
    }
}
