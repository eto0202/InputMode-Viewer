use gpui::{App, BorrowAppContext, Context, IntoElement, ParentElement, Styled, Task, Window};
use gpui_component::{
    Icon, IconName, IndexPath,
    button::Button,
    h_flex,
    list::{ListDelegate, ListItem, ListState},
};

use crate::common::{
    app_config::{AppConfig, PolicyMode},
    config,
};

pub struct ProcessListDelegate {
    all_items: Vec<String>,
    filtered_items: Vec<String>,
    current_query: String,
}

impl ProcessListDelegate {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            filtered_items: items.clone(),
            all_items: items,
            current_query: String::new(),
        }
    }

    // 親から検索文字を渡されて中身を絞り込む
    pub fn update_search(&mut self, query: &str) {
        self.current_query = query.to_string();
        let q = self.current_query.to_lowercase();
        if q.is_empty() {
            self.filtered_items = self.all_items.clone();
        } else {
            self.filtered_items = self
                .all_items
                .iter()
                .filter(|s| s.to_lowercase().contains(&q))
                .cloned()
                .collect();
        }
    }

    pub fn update_list(&mut self, items: Vec<String>) {
        self.all_items = items;

        let query = self.current_query.clone();
        self.update_search(&query);
    }
}

// 最小構成の実装
impl ListDelegate for ProcessListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        self.filtered_items.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let name = self.filtered_items.get(ix.row)?;
        Some(
            ListItem::new(ix).child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .child(name.clone())
                    .child(
                        Button::new(("p-btn-{}", ix.row))
                            .child(Icon::new(IconName::Plus))
                            .on_click({
                                let name = name.clone();
                                move |_, _, cx| {
                                    // ここでデータが変わったことを知らせる
                                    cx.update_global::<AppConfig, _>(|config, _| {
                                        if config.process_cfg.mode == PolicyMode::BlackList {
                                            config.process_cfg.blacklist.insert(name.as_str());
                                        } else {
                                            config.process_cfg.whitelist.insert(name.as_str());
                                        }
                                        let _ = config::save_config(config);
                                    });
                                }
                            }),
                    ),
            ),
        )
    }

    // --- その他は空実装 ---
    fn sections_count(&self, _: &App) -> usize {
        1
    }
    fn set_selected_index(
        &mut self,
        _: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        cx.notify();
    }
    fn render_section_header(
        &mut self,
        _: usize,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        Some(
            h_flex()
                .pb_1()
                .px_2()
                .gap_2()
                .text_sm()
                .child("Running Processes List"),
        )
    }
    fn render_section_footer(
        &mut self,
        _: usize,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        None::<gpui::Div>
    }
    fn perform_search(
        &mut self,
        _: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        Task::ready(())
    }
    fn confirm(&mut self, _: bool, _: &mut Window, _: &mut Context<ListState<Self>>) {}
    fn loading(&self, _: &App) -> bool {
        false
    }
    fn load_more_threshold(&self) -> usize {
        0
    }
    fn load_more(&mut self, _: &mut Window, _: &mut Context<ListState<Self>>) {}
}

pub struct CfgListDelegate {
    all_items: Vec<String>,
    filtered_items: Vec<String>,
    current_query: String,
}

impl CfgListDelegate {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            filtered_items: items.clone(),
            all_items: items,
            current_query: String::new(),
        }
    }

    // 親から検索文字を渡されて中身を絞り込む
    pub fn update_search(&mut self, query: &str) {
        self.current_query = query.to_string();
        let q = self.current_query.to_lowercase();
        if q.is_empty() {
            self.filtered_items = self.all_items.clone();
        } else {
            self.filtered_items = self
                .all_items
                .iter()
                .filter(|s| s.to_lowercase().contains(&q))
                .cloned()
                .collect();
        }
    }

    pub fn update_list(&mut self, items: Vec<String>) {
        self.all_items = items;

        let query = self.current_query.clone();
        self.update_search(&query);
    }
}

// 最小構成の実装
impl ListDelegate for CfgListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        self.filtered_items.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let name = self.filtered_items.get(ix.row)?;
        Some(
            ListItem::new(ix).child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .child(name.clone())
                    .child(
                        Button::new(("d-btn-{}", ix.row))
                            .child(Icon::new(IconName::Delete))
                            .on_click({
                                let name = name.clone();
                                move |_, _, cx| {
                                    cx.update_global::<AppConfig, _>(|config, _| {
                                        if config.process_cfg.mode == PolicyMode::BlackList {
                                            config.process_cfg.blacklist.remove(name.as_str());
                                        } else {
                                            config.process_cfg.whitelist.remove(name.as_str());
                                        }
                                        let _ = config::save_config(config);
                                    });
                                }
                            }),
                    ),
            ),
        )
    }

    // --- その他は空実装 ---
    fn sections_count(&self, _: &App) -> usize {
        1
    }
    fn set_selected_index(
        &mut self,
        _: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        cx.notify();
    }
    fn render_section_header(
        &mut self,
        _: usize,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        Some(h_flex().pb_1().px_2().gap_2().text_sm().child({
            if AppConfig::global(cx).process_cfg.mode == PolicyMode::BlackList {
                "Black List"
            } else {
                "White List"
            }
        }))
    }
    fn render_section_footer(
        &mut self,
        _: usize,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        None::<gpui::Div>
    }
    fn perform_search(
        &mut self,
        _: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        Task::ready(())
    }
    fn confirm(&mut self, _: bool, _: &mut Window, _: &mut Context<ListState<Self>>) {}
    fn loading(&self, _: &App) -> bool {
        false
    }
    fn load_more_threshold(&self) -> usize {
        0
    }
    fn load_more(&mut self, _: &mut Window, _: &mut Context<ListState<Self>>) {}
}
