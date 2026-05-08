use crate::{
    common::{
        app_config::{AppConfig, PolicyMode},
        config,
    },
    core::utils,
    ui::{components::list_components::delegate::CfgListDelegate, window::SettingsWindow},
};
use gpui::*;
use gpui::{Entity, SharedString};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, h_flex,
    input::{Input, InputEvent, InputState},
    list::{List, ListState},
    resizable::{h_resizable, resizable_panel},
    setting::{SettingField, SettingItem},
    v_flex,
};

use crate::ui::components::list_components::delegate::ProcessListDelegate;

pub struct ProcessList {
    pub search_input: Entity<InputState>,
    pub p_state: Entity<ListState<ProcessListDelegate>>,
    pub c_state: Entity<ListState<CfgListDelegate>>,
    pub _subscriptions: Vec<Subscription>,
}

impl ProcessList {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Self {
        let p_delegate = ProcessListDelegate::new(utils::get_running_process_names());

        let items = if AppConfig::global(cx).process_cfg.mode == PolicyMode::BlackList {
            AppConfig::global(cx)
                .process_cfg
                .blacklist
                .processes
                .clone()
        } else {
            AppConfig::global(cx)
                .process_cfg
                .whitelist
                .processes
                .clone()
        };
        let c_delegate = CfgListDelegate::new(items);

        let p_state = cx.new(|cx| ListState::new(p_delegate, window, cx).searchable(false));
        let c_state = cx.new(|cx| ListState::new(c_delegate, window, cx).searchable(false));

        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search process name..."));

        let p = p_state.clone();
        let c = c_state.clone();

        let _subscriptions = vec![
            cx.subscribe(&search_input, move |_, state, event, cx| match event {
                InputEvent::Change => p.update(cx, |this, cx| {
                    let text = state.read(cx).value();
                    this.delegate_mut().update_search(&text);
                }),
                InputEvent::Focus => println!("Focus"),
                _ => {}
            }),
            cx.subscribe(&search_input, move |_, state, event, cx| match event {
                InputEvent::Change => c.update(cx, |this, cx| {
                    let text = state.read(cx).value();
                    this.delegate_mut().update_search(&text);
                }),
                InputEvent::Focus => println!("Focus"),
                _ => {}
            }),
        ];

        // ListState は、自分が持っている Delegate の filtered_itemsしか見ていない
        // 各リストは自分の状態しか知らないため( global_mut で中身を書き換えても他のリストは知らない)、
        // ここで AppConfig の変更を待ち構えて、update_global された場合に各リストのデータを更新する
        cx.observe_global::<AppConfig>(|this, cx| {
            let config = AppConfig::global(cx);
            let latest_items: Vec<String> = if config.process_cfg.mode == PolicyMode::BlackList {
                config.process_cfg.blacklist.processes.clone()
            } else {
                config.process_cfg.whitelist.processes.clone()
            };

            this.process_list.c_state.update(cx, |state, _| {
                state.delegate_mut().update_list(latest_items);
            });

            cx.notify();
        })
        .detach();

        Self {
            search_input,
            p_state,
            c_state,
            _subscriptions,
        }
    }

    pub fn render_list(
        search_input: Entity<InputState>,
        p_state: Entity<ListState<ProcessListDelegate>>,
        c_state: Entity<ListState<CfgListDelegate>>,
        _: &mut Context<SettingsWindow>,
    ) -> Vec<SettingItem> {
        let p = p_state.clone();
        let c = c_state.clone();
        vec![
            SettingItem::new(
                "ProcessPolicy",
                SettingField::dropdown(
                    vec![
                        (PolicyMode::BlackList.as_ref().into(), "BlackList".into()),
                        (PolicyMode::WhiteList.as_ref().into(), "WhiteList".into()),
                    ],
                    |cx: &App| {
                        AppConfig::global(cx)
                            .process_cfg
                            .mode
                            .as_ref()
                            .to_string()
                            .into()
                    },
                    move |val: SharedString, cx: &mut App| {
                        let mode = val
                            .as_str()
                            .parse::<PolicyMode>()
                            .unwrap_or(PolicyMode::BlackList);

                        AppConfig::global_mut(cx).process_cfg.mode = mode;
                        let _ = config::save_config(AppConfig::global(cx));
                        p.update(cx, |_, cx| {
                            cx.notify();
                        });
                        c.update(cx, |_, cx| {
                            cx.notify();
                        });
                    },
                )
                .default_value(AppConfig::default().process_cfg.mode.as_ref().to_string()),
            )
            .description("ProcessPolicy: Default BlackList"),
            SettingItem::render(move |_options, _window, cx| {
                h_flex()
                    .w_full()
                    .justify_between()
                    .flex_wrap()
                    .gap_3()
                    .child(
                        v_flex()
                            .size_full()
                            .gap_4()
                            .child(
                                Input::new(&search_input)
                                    .cleanable(true)
                                    .prefix(Icon::new(IconName::Search).small()),
                            )
                            .child(
                                div()
                                    .h(px(400.))
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .rounded(cx.theme().radius)
                                    .child(
                                        h_resizable("resizable")
                                            .child(
                                                resizable_panel()
                                                    .size(px(300.))
                                                    .size_range(px(150.)..px(800.))
                                                    .child(List::new(&c_state)),
                                            )
                                            .child(
                                                resizable_panel()
                                                    .size(px(300.))
                                                    .size_range(px(150.)..px(800.))
                                                    .child(List::new(&p_state)),
                                            ),
                                    ),
                            ),
                    )
            }),
        ]
    }
}
