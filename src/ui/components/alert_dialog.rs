use crate::ui::prelude::*;

pub fn restart_alert_dialog(cx: &mut App, entity: WeakEntity<SettingsWindow>) -> impl IntoElement {
    let close = entity.clone();
    let action = entity.clone();
    let on_ok = entity.clone();
    let on_cancel = entity.clone();

    AlertDialog::new(cx)
        .on_cancel(move |_, _, cx| {
            if let Some(e) = on_cancel.upgrade() {
                e.update(cx, |this, cx| {
                    this.is_restart = false;
                    this.is_later = false;
                    cx.notify();
                });
            }
            true
        })
        .on_ok(move |_, _, cx| {
            if let Some(e) = on_ok.upgrade() {
                e.update(cx, |this, cx| {
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
                                        e.update(cx, |this, cx| {
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
                                        e.update(cx, |this, cx| {
                                            cx.notify();
                                            AppConfig::global_mut(cx).administrator = this.cur_admin;
                                            AppConfig::global_mut(cx).startup = this.cur_start;

                                            config::request_config_save(AppConfig::global(cx).clone());
                                        });
                                    }

                                }),
                        ),
                    ),
            )
    })
}
