#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ShowState {
    Hidden,
    Visible,
}

#[derive(PartialEq, Clone, Copy)]
pub enum AnimationAction {
    None,
    StartFadeIn, // 0.0 からフェードイン
    Refresh,     // 1.0 から再開
    Hide,        // 0.0 へ
}

impl ShowState {
    pub fn update(&mut self, displayed: bool) -> bool {
        match (displayed, *self) {
            (false, _) => {
                *self = ShowState::Hidden;
                false
            }
            (true, ShowState::Hidden) => {
                *self = ShowState::Visible;
                true
            }
            (true, ShowState::Visible) => false,
        }
    }

    pub fn is_action(&mut self, displayed: bool, refresh_requested: bool) -> AnimationAction {
        match (*self, displayed) {
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

    pub fn prepare_auto_hide(&mut self, displayed: bool) -> Option<bool> {
        match (displayed, *self) {
            (true, ShowState::Hidden) => {
                *self = ShowState::Visible;
                Some(false) // is_refresh = false (新規フェードイン)
            }
            (true, ShowState::Visible) => {
                Some(true) // is_refresh = true (表示維持・リフレッシュ)
            }
            (false, _) => {
                *self = ShowState::Hidden;
                None
            }
        }
    }
}
