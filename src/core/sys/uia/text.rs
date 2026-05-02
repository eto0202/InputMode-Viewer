pub const GLYPH_HIRAGANA: &str = "\u{e986}";
pub const GLYPH_HALF_ALPHA_1: &str = "\u{e97e}";
pub const GLYPH_HALF_ALPHA_2: &str = "\u{e982}";
pub const GLYPH_FULL_KATAKANA: &str = "\u{e987}";
pub const GLYPH_FULL_ALPHA: &str = "\u{e981}";
pub const GLYPH_HALF_KATAKANA: &str = "\u{e988}";

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    #[default]
    Unknown,
    Hiragana,
    HalfAlpha,
    FullKatakana,
    FullAlpha,
    HalfKatakana,
}

impl InputMode {
    pub fn new() -> Self {
        InputMode::default()
    }

    // グリフからModeを取得
    pub fn from_glyph(glyph: &str) -> Self {
        match glyph {
            GLYPH_HIRAGANA => Self::Hiragana,
            GLYPH_FULL_KATAKANA => Self::FullKatakana,
            GLYPH_FULL_ALPHA => Self::FullAlpha,
            GLYPH_HALF_KATAKANA => Self::HalfKatakana,

            GLYPH_HALF_ALPHA_1 | GLYPH_HALF_ALPHA_2 => Self::HalfAlpha,

            // 他のアイコン（Wi-Fi等）は無視
            _ => {
                log::debug!("Unknown IME Glyph detected: {:?}", glyph);
                Self::Unknown
            }
        }
    }

    // 表示用テキストを取得
    pub fn as_str_full(&self) -> &'static str {
        match self {
            Self::Hiragana => "ひらがな (あ)",
            Self::HalfAlpha => "半角英数 (A)",
            Self::FullKatakana => "全角カタカナ (カ)",
            Self::FullAlpha => "全角英数 (Ａ)",
            Self::HalfKatakana => "半角カタカナ (ｶ)",
            Self::Unknown => "",
        }
    }

    pub fn as_str_compact(&self) -> &'static str {
        match self {
            Self::Hiragana => "あ",
            Self::HalfAlpha => "A",
            Self::FullKatakana => "カ",
            Self::FullAlpha => "Ａ",
            Self::HalfKatakana => "ｶ",
            Self::Unknown => "",
        }
    }

    /// IMEがONかどうか
    pub fn is_on(&self) -> bool {
        !matches!(self, Self::HalfAlpha)
    }
}
