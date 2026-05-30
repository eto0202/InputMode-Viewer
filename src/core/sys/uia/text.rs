use crate::common::app_config::TextFormat;

// windows 11 のタスクバーで使われているのは Segoe Fluent Icons
// 日本語
pub const FULL_HIRAGANA: &str = "\u{e986}";
pub const FULL_KATAKANA: &str = "\u{e987}";
pub const HALF_KATAKANA: &str = "\u{e988}";

// 英数字
pub const HALF_ALPHA: &str = "\u{e97e}";
pub const FULL_ALPHA: &str = "\u{e97f}";

// 韓国語
pub const KOREAN: &str = "\u{e97d}";
pub const KEY_12_ON: &str = "\u{e980}";

// 中国語
pub const CHINESE_CHANGJIE: &str = "\u{e981}";
pub const QWERTY_ON: &str = "\u{e982}";
pub const QWERTY_OFF: &str = "\u{e983}";
pub const CHINESE_QUICK: &str = "\u{e984}";
pub const CHINESE_BO_PO_MO_FO: &str = "\u{e989}";
pub const CHINESE_PINYIN: &str = "\u{e98a}";
pub const JAPANESE: &str = "\u{e985}";

#[derive(Debug, Default, Clone, PartialEq)]
pub enum InputMode {
    #[default]
    Unknown,
    Layout(String), // キーボードレイアウト

    Hiragana,
    FullKatakana,
    HalfKatakana,

    HalfAlpha,
    FullAlpha,

    Korean,
    Key12on,

    ChineseChangjie,
    QwertyOn,
    QwertyOff,
    ChiniseQuick,
    ChiniseBoPoMoFo,
    ChinisePinyin,
    Japanese,
}

impl InputMode {
    pub fn new() -> Self {
        InputMode::default()
    }

    // グリフからModeを取得
    pub fn from_glyph(glyph: &str) -> Self {
        match glyph {
            FULL_HIRAGANA => Self::Hiragana,
            FULL_KATAKANA => Self::FullKatakana,
            HALF_KATAKANA => Self::HalfKatakana,

            FULL_ALPHA => Self::FullAlpha,
            HALF_ALPHA => Self::HalfAlpha,

            KOREAN => Self::Korean,
            KEY_12_ON => Self::Key12on,

            CHINESE_CHANGJIE => Self::ChineseChangjie,
            QWERTY_ON => Self::QwertyOn,
            QWERTY_OFF => Self::QwertyOff,
            CHINESE_QUICK => Self::ChiniseQuick,
            CHINESE_BO_PO_MO_FO => Self::ChiniseBoPoMoFo,
            CHINESE_PINYIN => Self::ChinisePinyin,
            JAPANESE => Self::Japanese,

            // 他のアイコン（Wi-Fi等）は無視
            // どうやらキーボードレイアウトの情報は同じ場所に入るらしく
            // ENG のように取得されている
            _ => {
                log::debug!("Unknown IME Glyph detected: {:?}", glyph);
                // 文字列の最初の1文字を取り出して文字コードを確認
                if let Some(c) = glyph.chars().next() {
                    // Unicodeの私的領域の範囲かチェック
                    // 一般的なPUA範囲: U+E000 ～ U+F8FF
                    if ('\u{e000}'..='\u{f8ff}').contains(&c) {
                        // アイコンフォント
                        Self::Unknown
                    } else {
                        // "ENG" などの文字列
                        Self::Layout(glyph.to_string())
                    }
                } else {
                    Self::Unknown
                }
            }
        }
    }

    // 表示用テキストを取得
    pub fn as_str(&self, text_format: TextFormat) -> &str {
        match text_format {
            TextFormat::Full => self.as_str_full(),
            TextFormat::Compact => self.as_str_compact(),
        }
    }

    fn as_str_full(&self) -> &str {
        match self {
            Self::Hiragana => "ひらがな (あ)",
            Self::FullKatakana => "全角カタカナ (カ)",
            Self::HalfKatakana => "半角カタカナ (ｶ)",

            Self::HalfAlpha => "半角英数 (A)",
            Self::FullAlpha => "全角英数 (Ａ)",

            Self::Korean => "한국어 (한)",
            Self::Key12on => "12키 (가)",

            Self::ChineseChangjie => "倉頡 (倉)",
            Self::QwertyOn => "中文 (中)",
            Self::QwertyOff => "英文 (英)",
            Self::ChiniseBoPoMoFo => "注音 (ㄅ)",
            Self::ChiniseQuick => "速成 (速)",
            Self::ChinisePinyin => "简体 (简)",
            Self::Japanese => "日文 (日)",

            Self::Unknown => "",
            Self::Layout(s) => s.as_str(),
        }
    }

    fn as_str_compact(&self) -> &str {
        match self {
            Self::Hiragana => "あ",
            Self::FullKatakana => "カ",
            Self::HalfKatakana => "ｶ",

            Self::HalfAlpha => "A",
            Self::FullAlpha => "Ａ",

            Self::Korean => "한",
            Self::Key12on => "가",

            Self::ChineseChangjie => "倉",
            Self::QwertyOn => "中",
            Self::QwertyOff => "英",
            Self::ChiniseBoPoMoFo => "ㄅ",
            Self::ChiniseQuick => "速",
            Self::ChinisePinyin => "简",
            Self::Japanese => "日",

            Self::Unknown => "",
            Self::Layout(s) => s.as_str(),
        }
    }

    /// IMEがONかどうか
    pub fn is_on(&self) -> bool {
        !matches!(self, Self::HalfAlpha | Self::QwertyOff | Self::Layout(_))
    }
}
