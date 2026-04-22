use crate::core::app::prelude::*;

// 「隠す」「フェードイン中」「表示中」の3つの状態で管理し、アニメーションを実装
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ShowState {
    Hidden,
    FadeIn {
        start_at: Instant,
        duration: Duration,
    },
    Visible,
}

impl ShowState {
    pub fn update(
        &mut self,
        fade_duration: Duration,
        should_show: bool,
        target_opacity: f32,
    ) -> (f32, bool) {
        match (should_show, &self) {
            // 非表示にすべき時
            (false, _) => {
                *self = ShowState::Hidden;
                (0.0, false) // 透明度は0 次の再描画は不要
            }

            // 非表示からフェードイン開始
            (true, ShowState::Hidden) => {
                *self = ShowState::FadeIn {
                    start_at: Instant::now(),
                    duration: fade_duration,
                };
                (0.0, true) // 透明度は0 アニメーション開始のため再描画が必要
            }

            // フェードイン中
            (true, ShowState::FadeIn { start_at, duration }) => {
                let elapsed = start_at.elapsed();
                // 経過時間 ÷ 指定ミリ秒 で進捗率（0.0〜1.0）を出す。1.0までループし徐々に濃く。
                let progress = (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.0);
                let current_opacity = progress * target_opacity;

                if progress >= 1.0 {
                    *self = ShowState::Visible;
                    (target_opacity, false) // 完了
                } else {
                    (current_opacity, true) // まだ途中なので再描画が必要
                }
            }

            // すでに表示中
            (true, ShowState::Visible) => {
                (target_opacity, false) // 目標の透明度、アニメーションはしていない
            }
        }
    }
}
