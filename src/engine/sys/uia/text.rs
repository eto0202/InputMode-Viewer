use std::borrow::Cow;

use crate::common::TextFormat;



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
pub enum InputMode<'a> {
    #[default]
    Unknown,
    Layout(Cow<'a, str>), // キーボードレイアウト

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

impl<'a> InputMode<'a> {
    pub fn new() -> Self {
        InputMode::default()
    }

    // グリフからModeを取得
    pub fn from_glyph<S>(glyph: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        let glyph_cow = glyph.into();

        match glyph_cow.as_ref() {
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

            // 他のアイコンは無視
            // どうやらキーボードレイアウトの情報は同じ場所に入るらしくENG のように取得されている
            _ => {
                tracing::debug!("Unknown IME Glyph detected: {:?}", glyph_cow);
                // 文字列の最初の1文字を取り出して文字コードを確認
                if let Some(c) = glyph_cow.chars().next() {
                    // Unicodeの私的領域の範囲かチェック
                    // 一般的なPUA範囲: U+E000 ～ U+F8FF
                    if ('\u{e000}'..='\u{f8ff}').contains(&c) {
                        // アイコンフォント
                        Self::Unknown
                    } else {
                        // "ENG" などの文字列
                        Self::Layout(glyph_cow)
                    }
                } else {
                    Self::Unknown
                }
            }
        }
    }

    // 表示用テキストを取得
    // 動的に文字列を生成しないため 'a をそのまま返せる
    pub fn as_str(&self, text_format: TextFormat) -> Cow<'a, str> {
        match text_format {
            TextFormat::Full => self.as_str_full(),
            TextFormat::Compact => self.as_str_compact(),
        }
    }

    fn as_str_full(&self) -> Cow<'a, str> {
        match self {
            Self::Hiragana => Cow::Borrowed("ひらがな (あ)"),
            Self::FullKatakana => Cow::Borrowed("全角カタカナ (カ)"),
            Self::HalfKatakana => Cow::Borrowed("半角カタカナ (ｶ)"),

            Self::HalfAlpha => Cow::Borrowed("半角英数 (A)"),
            Self::FullAlpha => Cow::Borrowed("全角英数 (Ａ)"),

            Self::Korean => Cow::Borrowed("한국어 (한)"),
            Self::Key12on => Cow::Borrowed("12키 (가)"),

            Self::ChineseChangjie => Cow::Borrowed("倉頡 (倉)"),
            Self::QwertyOn => Cow::Borrowed("中文 (中)"),
            Self::QwertyOff => Cow::Borrowed("英文 (英)"),
            Self::ChiniseBoPoMoFo => Cow::Borrowed("注音 (ㄅ)"),
            Self::ChiniseQuick => Cow::Borrowed("速成 (速)"),
            Self::ChinisePinyin => Cow::Borrowed("简体 (简)"),
            Self::Japanese => Cow::Borrowed("日文 (日)"),

            Self::Unknown => Cow::Borrowed(""),
            // 内部の Cow をクローンして返します（Borrowed ならポインタのコピーのみで軽量）
            Self::Layout(s) => s.clone(),
        }
    }

    fn as_str_compact(&self) -> Cow<'a, str> {
        match self {
            Self::Hiragana => Cow::Borrowed("あ"),
            Self::FullKatakana => Cow::Borrowed("カ"),
            Self::HalfKatakana => Cow::Borrowed("ｶ"),

            Self::HalfAlpha => Cow::Borrowed("A"),
            Self::FullAlpha => Cow::Borrowed("Ａ"),

            Self::Korean => Cow::Borrowed("한"),
            Self::Key12on => Cow::Borrowed("가"),

            Self::ChineseChangjie => Cow::Borrowed("倉"),
            Self::QwertyOn => Cow::Borrowed("中"),
            Self::QwertyOff => Cow::Borrowed("英"),
            Self::ChiniseBoPoMoFo => Cow::Borrowed("ㄅ"),
            Self::ChiniseQuick => Cow::Borrowed("速"),
            Self::ChinisePinyin => Cow::Borrowed("简"),
            Self::Japanese => Cow::Borrowed("日"),

            Self::Unknown => Cow::Borrowed(""),
            // 内部の Cow をクローンして返す
            Self::Layout(s) => s.clone(),
        }
    }

    /// IMEがONかどうか
    pub fn is_on(&self) -> bool {
        !matches!(self, Self::HalfAlpha | Self::QwertyOff | Self::Layout(_))
    }
}
