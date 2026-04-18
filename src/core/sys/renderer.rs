use std::f32;

use windows::{
    Win32::{
        Foundation::*,
        Graphics::{
            Direct2D::{
                Common::{
                    D2D_RECT_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
                },
                *,
            },
            Direct3D::*,
            Direct3D11::*,
            DirectComposition::*,
            DirectWrite::{
                DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_WEIGHT_MEDIUM, DWRITE_MEASURING_MODE_NATURAL,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
                DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat,
            },
            Dxgi::{Common::*, *},
        },
    },
    core::*,
};

/*
1.D3D11
    GPUという巨大な調理場を確保する担当。
2.DXGI
    「できた料理（画像）を、どうやってお客さん（画面）に届けるか」という配送ルート（スワップチェーン）を作る担当。
3.Direct2D
    「実際に絵を描く」担当。
4.DirectWrite
    「綺麗な文字のデザイン」だけを専門に考える担当。
5.DirectComposition
    描いた絵を「シール」のようにウィンドウにペタッと貼り付けたり、透かしを入れたりする最新の担当。
*/
use crate::{common::app_config::WindowStyle, core::sys::uia::text::InputMode};

#[derive(Debug)]
pub struct DCompRenderer {
    pub d2d_factory: ID2D1Factory1,
    pub dw_factory: IDWriteFactory,
    pub d2d_context: ID2D1DeviceContext, // 実際の描画命令を出すためのコンテキスト
    pub swap_chain: IDXGISwapChain1,     // 描画した画像を画面に表示するためのバッファ管理機構
    pub font_brush: ID2D1SolidColorBrush,
    pub bg_brush: ID2D1SolidColorBrush,
    pub format: IDWriteTextFormat,

    // Windowsのデスクトップコンポジターとやり取りし、描画内容をウィンドウに貼り付けるためのもの
    pub dcomp_device: IDCompositionDevice,
    pub dcomp_visual: IDCompositionVisual,
    pub dcomp_target: IDCompositionTarget,

    // ウィンドウ全体の不透明度などを制御するためのエフェクト設定
    pub dcomp_effect_group: IDCompositionEffectGroup,

    // 現在のスタイルのキャッシュ
    pub current_font_size: f32,
    pub current_bg_color: D2D1_COLOR_F,
}

impl DCompRenderer {
    pub fn new(
        hwnd: HWND,
        input_mode: InputMode,
        style: &WindowStyle,
        scale: f64,
    ) -> anyhow::Result<(Self, f32, f32)> {
        unsafe {
            // D3D11 Deviceの作成
            // 全ての基盤となるGPUとの対話窓口
            // D3D11_CREATE_DEVICE_BGRA_SUPPORTがないと、後でDirect2Dを繋げようとした時にエラーで落ちる
            let mut d3d_device: Option<ID3D11Device> = None;
            D3D11CreateDevice(
                None,                             // 使用するグラボ。Noneはメインのグラボ
                D3D_DRIVER_TYPE_HARDWARE,         // ハードウェア(GPU)を使う宣言
                HMODULE::default(), // ソフトウェアレンダラを使う場合のパス（基本使わない）
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, // Direct2Dと連携するならこのフラグが必須
                None,               // サポートしたい機能レベル（Noneなら最新を自動選択）
                D3D11_SDK_VERSION,  // SDKのバージョン（おまじない）
                Some(&mut d3d_device), // 生成されたデバイスの受け取り先
                None,               // 実際に決まった機能レベルの受け取り先
                None,               // デバイスコンテキストの受け取り先
            )?;
            let d3d_device = d3d_device.unwrap();
            let dxgi_device: IDXGIDevice = d3d_device.cast()?;

            // D2D Deviceの作成
            // D3D11の上で動作する、2D描画（ベクターグラフィックス）用のインターフェース
            let d2d_factory: ID2D1Factory1 =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let d2d_device = d2d_factory.CreateDevice(&dxgi_device)?;
            let d2d_context = d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;

            // DXGISwapChain(Flip Model)の作成
            // 描画結果を画面に送り出すためのダブルバッファ
            let dxgi_factory: IDXGIFactory2 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;

            // DirectCompositionのセットアップ
            let dcomp_device: IDCompositionDevice = DCompositionCreateDevice(&dxgi_device)?;
            // 特定のウィンドウを描画対象に
            let dcomp_target = dcomp_device.CreateTargetForHwnd(hwnd, BOOL(1).as_bool())?;
            // 描画されるレイヤー。ここにswap_chainをセット
            let dcomp_visual = dcomp_device.CreateVisual()?;

            // エフェクトグループを作成
            // 透明度などを制御するための入れ物
            let dcomp_effect_group = dcomp_device.CreateEffectGroup()?;
            // Visual にエフェクトを紐付ける（初期値は不透明度 1.0 にしておく）
            dcomp_visual.SetEffect(&dcomp_effect_group)?;
            dcomp_target.SetRoot(&dcomp_visual)?;

            // テキスト作成
            let dw_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            // テキストのフォントやサイズ、整列などの定義
            // w!はUTF-16のワイド文字列に変換するマクロ
            let format = dw_factory.CreateTextFormat(
                w!("Noto Sans JP"), // フォント名（Windowsにインストールされている必要あり。無い場合は代替フォント）
                None,               // フォントコレクション（Noneはシステム標準）
                DWRITE_FONT_WEIGHT_MEDIUM, // 太さ
                DWRITE_FONT_STYLE_NORMAL, // スタイル（イタリックなど）
                DWRITE_FONT_STRETCH_NORMAL, // 文字幅の伸縮
                style.font_size,    // フォントサイズ（DIP単位）
                w!("ja-jp"),        // 言語
            )?;

            format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            // 初期文字列からサイズを計算する (calc_metrics と同等の処理)
            let text: Vec<u16> = input_mode
                .as_str()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let text_layout = dw_factory.CreateTextLayout(&text, &format, f32::MAX, f32::MAX)?;

            let mut metrics = std::mem::zeroed();
            text_layout.GetMetrics(&mut metrics)?;

            // パディングを足して正確なスワップチェーンのサイズを算出
            let logical_w = metrics.width + style.padding * 2.0;
            let logical_h = metrics.height + style.padding * 2.0;

            let physical_w = (logical_w * scale as f32) as u32;
            let physical_h = (logical_h * scale as f32) as u32;

            let swap_chain_desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: physical_w,                  // 画面の幅
                Height: physical_h,                 // 画面の高さ
                Format: DXGI_FORMAT_B8G8R8A8_UNORM, // 色の並び(Blue, Green, Red, Alpha)
                Stereo: BOOL(0),                    // 3D立体視にするか（基本0）
                SampleDesc: DXGI_SAMPLE_DESC {
                    // アンチエイリアスの設定
                    Count: 1, // 1なら無効（2D描画はD2D側でやるので1）
                    Quality: 0,
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT, // このバッファを何に使うか（出力用）
                BufferCount: 2, // ダブルバッファ（描画中と表示中の2枚持つ）
                Scaling: DXGI_SCALING_STRETCH, // ウィンドウサイズが変わった時の引き伸ばし設定
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD, // 最新の高速な画面切り替え方式
                AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED, // 透過ウィンドウにするならこれ
                Flags: 0,
            };
            // CreateSwapChainForCompositionを使っているのは、DirectCompositionと連携するため
            let swap_chain =
                dxgi_factory.CreateSwapChainForComposition(&d3d_device, &swap_chain_desc, None)?;

            // SwapChainをVisualの内容にセット
            dcomp_visual.SetContent(&swap_chain)?;

            let font_brush = d2d_context.CreateSolidColorBrush(&style.font_color, None)?;
            let bg_brush = d2d_context.CreateSolidColorBrush(&style.bg_color, None)?;

            let dpi = (scale * 96.0) as f32;
            d2d_context.SetDpi(dpi, dpi);

            dcomp_device.Commit()?;

            let renderer = Self {
                d2d_factory,
                dw_factory,
                d2d_context,
                swap_chain,
                font_brush,
                bg_brush,
                format,
                dcomp_device,
                dcomp_visual,
                dcomp_target,
                dcomp_effect_group,
                current_bg_color: style.bg_color,
                current_font_size: style.font_size,
            };

            Ok((renderer, logical_w, logical_h))
        }
    }

    // 毎フレーム、または再描画が必要な時に呼ばれる関数
    pub fn draw(
        &self,
        input_mode: InputMode,
        width: f32,
        height: f32,
        padding: f32,
    ) -> anyhow::Result<()> {
        unsafe {
            // 1. SwapChainのバッファをD2Dの描き先に設定
            // 次に書き込むための画用紙（DXGI Surface）を取得
            let dxgi_surface: IDXGISurface = self.swap_chain.GetBuffer(0)?;

            let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                ..Default::default()
            };
            // Direct2Dが扱える形式に変換し、d2d_context.SetTargetでセット
            let d2d_bitmap = self
                .d2d_context
                .CreateBitmapFromDxgiSurface(&dxgi_surface, Some(&bitmap_props))?;

            self.d2d_context.SetTarget(&d2d_bitmap);
            // 描画開始
            self.d2d_context.BeginDraw();
            // アンチエイリアス
            self.d2d_context
                .SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            // 背景を透明でクリア
            self.d2d_context.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            // 背景を角丸矩形で描画
            let rect = D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: width,
                bottom: height,
            };

            let rounded_rect = D2D1_ROUNDED_RECT {
                rect,
                radiusX: 2.0, // 角丸の半径
                radiusY: 2.0,
            };
            self.d2d_context
                .FillRoundedRectangle(&rounded_rect, &self.bg_brush);

            // 文字列を取得
            // Rustの文字列はUTF-8、WindowsAPIはUTF-16。C言語の名残で最後は0で終わるというルール
            let text: Vec<u16> = input_mode
                .as_str()
                .encode_utf16()
                .chain(std::iter::once(0)) // ヌル終端
                .collect();

            // paddingを加味した描画領域
            let text_rect = D2D_RECT_F {
                left: padding,
                top: padding,
                right: width as f32 - padding,
                bottom: height as f32 - padding,
            };

            // 中央に描画
            self.d2d_context.DrawText(
                &text,
                &self.format,
                &text_rect,
                &self.font_brush,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // ここでGPUに描画命令
            self.d2d_context.EndDraw(None, None)?;

            // 描画したバッファを画面に表示
            self.swap_chain.Present(1, DXGI_PRESENT::default()).ok()?;
            // DirectComposition側に「準備ができたので合成して表示して」と伝える
            self.dcomp_device.Commit()?;

            // ターゲットを外す
            self.d2d_context.SetTarget(None);
            Ok(())
        }
    }

    // 透明度操作
    pub fn set_visibility(&self, opacity: f32) -> anyhow::Result<()> {
        unsafe {
            self.dcomp_effect_group.SetOpacity2(opacity)?;
            self.dcomp_device.Commit()?;
            Ok(())
        }
    }

    // todo:実際のフォントサイズを計算
    pub fn calc_metrics(&self, input_mode: InputMode) -> anyhow::Result<(f32, f32)> {
        unsafe {
            let text: Vec<u16> = input_mode
                .as_str()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            // 無限の大きさを指定して、自然な改行位置を測る
            let text_layout =
                self.dw_factory
                    .CreateTextLayout(&text, &self.format, f32::MAX, f32::MAX)?;

            let mut metrics = std::mem::zeroed();
            text_layout.GetMetrics(&mut metrics)?;

            // DIP単位での幅と高さが取得できる
            Ok((metrics.width, metrics.height))
        }
    }

    pub fn update_config(&mut self, style: &WindowStyle) -> anyhow::Result<()> {
        unsafe {
            // 色の更新
            self.font_brush.SetColor(&style.font_color);
            self.bg_brush.SetColor(&style.bg_color);
            self.current_bg_color = style.bg_color;

            // フォントサイズが変わった場合のみ、TextFormatを再生成
            if (self.current_font_size - style.font_size).abs() > f32::EPSILON {
                let new_format = self.dw_factory.CreateTextFormat(
                    w!("Noto Sans JP"),
                    None,
                    DWRITE_FONT_WEIGHT_MEDIUM,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    style.font_size,
                    w!("ja-jp"),
                )?;

                new_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
                new_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;

                self.format = new_format;
                self.current_font_size = style.font_size;
            }
        }
        Ok(())
    }

    pub fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        // サイズがゼロの時はリサイズしない
        if width == 0 || height == 0 {
            return Ok(());
        }

        unsafe {
            // D2DコンテキストがSwapChainのバッファを掴んだままだと ResizeBuffers が失敗する
            self.d2d_context.SetTarget(None);

            // バッファのリサイズ
            self.swap_chain.ResizeBuffers(
                0, // 0 = 現在のバッファ数(2)を維持
                width,
                height,
                DXGI_FORMAT_UNKNOWN,
                DXGI_SWAP_CHAIN_FLAG(0),
            )?;

            // DCompのVisualにバッファを再セットし、Commitする
            self.dcomp_visual.SetContent(&self.swap_chain)?;
            self.dcomp_device.Commit()?;
            self.d2d_context.SetTarget(None);
        }
        Ok(())
    }
}
