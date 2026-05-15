#[derive(PartialEq, Clone, Copy)]
pub enum ShowState {
    Hidden,
    Visible,
}

impl ShowState {
    // 描画が必要なタイミングかどうかを判定
    pub fn update(&mut self, should_show: bool, refresh_requested: bool) -> AnimationAction {
        match (*self, should_show) {
            // 非表示 -> 表示
            (ShowState::Hidden, true) => {
                *self = ShowState::Visible;
                AnimationAction::StartFadeIn
            }
            // 表示中 -> 表示中 (操作があった場合のリフレッシュ)
            (ShowState::Visible, true) if refresh_requested => AnimationAction::Refresh,
            // 表示 -> 非表示
            (ShowState::Visible, false) => {
                *self = ShowState::Hidden;
                AnimationAction::Hide
            }
            _ => AnimationAction::None,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum AnimationAction {
    None,
    StartFadeIn, // 0.0 からフェードイン
    Refresh,     // 1.0 から再開
    Hide,        // 0.0 へ
}
