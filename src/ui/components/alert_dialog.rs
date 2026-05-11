use gpui::{
    App, AppContext, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement, Render, Styled, Window, div,
};

use gpui_component::{
    ActiveTheme,WindowExt as _,
    button::{Button,ButtonVariants},
    dialog::{
        AlertDialog, DialogAction, DialogClose, DialogDescription, DialogFooter,
        DialogHeader, DialogTitle,
    },
    v_flex,
};

pub struct RestartAlertDialog {
    focus_handle: FocusHandle,
}

impl RestartAlertDialog {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Render for RestartAlertDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("restart-alert-dialog")
            .track_focus(&self.focus_handle)
            .size_full()
            .child(
                v_flex().gap_6().child(
                    div().child(
                        AlertDialog::new(cx)
                            .trigger(
                                Button::new("restart")
                                    .outline()
                                    .label("Restart Application"),
                            )
                            .on_cancel(|_, window, cx| {
                                window.push_notification("Restart postponed", cx);
                                true
                            })
                            .on_ok(|_, window, cx| {
                                window.push_notification("Restarting now...", cx);
                                true
                            })
                            .content(|content, _, cx| {
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
                                            .bg(cx.theme().muted)
                                            .child(
                                                DialogClose::new().child(
                                                    Button::new("later")
                                                        .flex_1()
                                                        .outline()
                                                        .label("Later"),
                                                ),
                                            )
                                            .child(
                                                DialogAction::new().child(
                                                    Button::new("restart-now")
                                                        .flex_1()
                                                        .primary()
                                                        .label("Restart Now"),
                                                ),
                                            ),
                                    )
                            }),
                    ),
                ),
            )
    }
}
