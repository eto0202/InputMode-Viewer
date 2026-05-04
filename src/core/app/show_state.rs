

// 「隠す」「フェードイン中」「表示中」の3つの状態で管理し、アニメーションを実装
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ShowState {
    Hidden,
    Visible,
}

impl ShowState {
    pub fn update(&mut self, displayed: bool) -> bool {
        match (displayed, *self) {
            // 非表示にすべき時
            (false, _) => {
                *self = ShowState::Hidden;
                false // 透明度は0 次の再描画は不要
            }
            // 非表示から表示へ：フェードイン命令を出し、状態をVisibleへ
            (true, ShowState::Hidden) => {
                *self = ShowState::Visible;
                true
            }

            // 表示中から非表示へ：フェードアウト命令を出し、状態をHiddenへ
            (true, ShowState::Visible) => false,
        }
    }
}
