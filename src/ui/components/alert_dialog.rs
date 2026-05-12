use gpui::{App, IntoElement, ParentElement, Styled, WeakEntity};

use gpui_component::{
    ActiveTheme,
    button::{Button, ButtonVariants},
    dialog::{
        AlertDialog, DialogAction, DialogClose, DialogDescription, DialogFooter, DialogHeader,
        DialogTitle,
    },
};

use crate::{
    common::{app_config::AppConfig, config},
    ui::window::SettingsWindow,
};

pub fn restart_alert_dialog(cx: &mut App, entity: WeakEntity<SettingsWindow>) -> impl IntoElement {
    let close = entity.clone();
    let action = entity.clone();
    let on_ok = entity.clone();
    let on_cancel = entity.clone();

    AlertDialog::new(cx)
        .on_cancel(move |_, _, cx| {
            if let Some(e) = on_cancel.upgrade() {
                let _ = e.update(cx, |this, cx| {
                    this.is_restart = false;
                    this.is_later = false;
                    cx.notify();
                });
            }
            true
        })
        .on_ok(move |_, _, cx| {
            if let Some(e) = on_ok.upgrade() {
                let _ = e.update(cx, |this, cx| {
                    this.is_restart = false;
                    this.is_later = false;
                    cx.notify();
                });
            }
            true
        })
        .content(move |content, _, cx| {
            let close = close.clone();
            let action = action.clone();

            content
            .child(
                DialogHeader::new()
                    .child(DialogTitle::new().child("Restart Application"))
                    .child(DialogDescription::new().child(
                        "Important settings have been changed.\nPlease restart the application to apply the changes.",
                    )),
            )
            .child(
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("later")
                                .flex_1()
                                .outline()
                                .label("Later")
                                .bg(cx.theme().muted)
                                .on_click(move |_, _, cx| {
                                    if let Some(e) = close.upgrade() {
                                        let _ = e.update(cx, |this, cx| {
                                            this.is_restart = false;
                                            this.is_later = true;
                                            cx.notify();
                                        });
                                    }
                                }),
                        ),
                    )
                    .child(
                        DialogAction::new().child(
                            Button::new("restart-now")
                                .flex_1()
                                .primary()
                                .label("Restart Now")
                                .on_click(move |_, _, cx| {
                                    if let Some(e) = action.upgrade() {
                                        let _ = e.update(cx, |this, cx| {
                                            this.is_restart = false;
                                            this.is_later = false;
                                            cx.notify();
                                            let _ = config::save_config(AppConfig::global(cx));
                                        });
                                    }

                                }),
                        ),
                    ),
            )
    })
}
