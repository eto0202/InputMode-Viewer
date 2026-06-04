use crate::{
    common::app_config::{TextFormat, WindowStyle},
    core::sys::uia::text::InputMode,
};
use anyhow::Context;
use std::{f32, time::Duration};
use windows::{
    Foundation::TimeSpan,
    System::DispatcherQueueController,
    UI::Composition::{
        CompositionPropertySet, Compositor, ContainerVisual, Desktop::DesktopWindowTarget,
        ExpressionAnimation, SpriteVisual,
    },
    Win32::{
        Foundation::{HANDLE, *},
        Graphics::{
            Direct2D::{
                Common::{
                    D2D_RECT_F, D2D1_ALPHA_MODE, D2D1_ALPHA_MODE_IGNORE,
                    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
                },
                ID2D1DeviceContext, ID2D1Factory1, ID2D1SolidColorBrush, *,
            },
            Direct3D::*,
            Direct3D11::{D3D11CreateDevice, ID3D11Device, *},
            DirectWrite::{
                DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_WEIGHT_MEDIUM, DWRITE_MEASURING_MODE_NATURAL,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
                DWRITE_TEXT_METRICS, DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat,
            },
            Dxgi::{Common::*, IDXGIDevice, IDXGIFactory2, IDXGISwapChain1, *},
        },
        System::{
            Threading::WaitForSingleObject,
            WinRT::{
                Composition::{ICompositorDesktopInterop, ICompositorInterop},
                CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT,
                DispatcherQueueOptions,
            },
        },
    },
    core::*,
};
use windows_numerics::{Vector2, Vector3};

#[derive(Debug)]
pub struct DCompRenderer {
    // 基盤 (DirectX / Direct2D)
    pub d3d_device: ID3D11Device,
    pub dxgi_factory: IDXGIFactory2,
    pub d2d_context: ID2D1DeviceContext,
    pub swap_chain: IDXGISwapChain1,
    pub waitable_object: HANDLE,

    // タイポグラフィ
    pub dw_factory: IDWriteFactory,
    pub format: IDWriteTextFormat,
    pub font_brush: ID2D1SolidColorBrush,
    pub bg_brush: ID2D1SolidColorBrush,

    // WinRT Composition
    pub compositor: Compositor,
    pub desktop_target: DesktopWindowTarget,
    pub root_visual: ContainerVisual,
    pub sprite_visual: SpriteVisual,

    // 現在のスタイルのキャッシュ
    pub current_font_size: f32,
    pub current_bg_color: D2D1_COLOR_F,
    pub current_alpha_mode: D2D1_ALPHA_MODE,
    // 作り直しフラグ（Noneなら作り直し不要）
    pub pending_alpha_recreation: Option<D2D1_ALPHA_MODE>,

    // 式アニメーション
    pub property_set: CompositionPropertySet,
    pub mouse_expr: ExpressionAnimation,
}

#[derive(Debug, Clone)]
pub struct RendererController {
    pub sprite_visual: SpriteVisual,
    pub property_set: CompositionPropertySet,
    pub mouse_expr: ExpressionAnimation,
}

impl RendererController {
    pub fn set_position(&self, x: f32, y: f32) -> anyhow::Result<()> {
        self.sprite_visual
            .SetOffset(Vector3 { X: x, Y: y, Z: 0.0 })?;
        Ok(())
    }

    pub fn mouse_expr_start(&self) -> anyhow::Result<()> {
        self.sprite_visual
            .StartAnimation(h!("Offset"), &self.mouse_expr)?;
        Ok(())
    }

    // マウス追従
    pub fn mouse_tracking(&self, tx: i32, ty: i32) -> anyhow::Result<()> {
        // 共有変数の値を更新
        self.property_set.InsertVector3(
            h!("MousePos"),
            Vector3 {
                X: tx as f32,
                Y: ty as f32,
                Z: 0.0,
            },
        )?;
        Ok(())
    }
}

impl DCompRenderer {
    pub fn get_controller(&self) -> RendererController {
        RendererController {
            sprite_visual: self.sprite_visual.clone(),
            property_set: self.property_set.clone(),
            mouse_expr: self.mouse_expr.clone(),
        }
    }

    pub fn new(
        hwnd: HWND,
        mode: InputMode,
        style: &WindowStyle,
        scale: f64,
        transparent: bool,
    ) -> anyhow::Result<Self> {
        // 基盤
        let (d3d_device, dxgi_device, dxgi_factory, _, d2d_context) = create_graphics_foundation()?;
        tracing::info!("Graphics Foundation (D3D, DXGI, D2D) OK");

        // Composition層 (WinRT)
        let (compositor, desktop_target, root_visual, sprite_visual) =
            create_composition_layer(hwnd, &dxgi_device)?;
        tracing::info!("DirectComposition OK");

        // タイポグラフィ
        let (dw_factory, format, lw, lh) = create_typography(style, mode)?;
        let font_brush = unsafe { d2d_context.CreateSolidColorBrush(&style.font_color, None)? };
        let bg_brush = unsafe { d2d_context.CreateSolidColorBrush(&style.bg_color, None)? };
        tracing::info!("Typography (DirectWrite) OK");

        // スワップチェーン作成
        let width = (lw * scale as f32) as u32;
        let height = (lh * scale as f32) as u32;
        let swap_chain = create_swap_chain(&dxgi_factory, &d3d_device, width, height, transparent)?;
        tracing::info!("Presentation (SwapChain, Brushes) OK");

        // Vcyncの設定
        let waitable_object = unsafe {
            let sc2: IDXGISwapChain2 = swap_chain.cast()?;
            dxgi_device
                .cast::<IDXGIDevice1>()?
                .SetMaximumFrameLatency(1)?;
            sc2.GetFrameLatencyWaitableObject()
        };
        tracing::info!("WaitableObject OK");

        let dpi = (scale * 96.0) as f32;
        unsafe {
            d2d_context.SetDpi(dpi, dpi);
        };

        let current_alpha_mode = if transparent {
            D2D1_ALPHA_MODE_PREMULTIPLIED
        } else {
            D2D1_ALPHA_MODE_IGNORE
        };
        let pending_alpha_recreation = Some(current_alpha_mode);

        let (property_set, mouse_expr) = create_expression_animation(&compositor, &sprite_visual)?;
        tracing::info!("CompositionPropertySet OK");

        connect_swap_chain_to_visual(&compositor, &sprite_visual, &swap_chain)?;
        tracing::info!("Connect swap chain to visual");

        let renderer = Self {
            d3d_device,
            dxgi_factory,
            d2d_context,
            swap_chain,
            waitable_object,
            dw_factory,
            format,
            font_brush,
            bg_brush,
            compositor,
            desktop_target,
            root_visual,
            sprite_visual,
            current_bg_color: style.bg_color,
            current_font_size: style.font_size,
            current_alpha_mode,
            pending_alpha_recreation,
            property_set,
            mouse_expr,
        };

        Ok(renderer)
    }

    // 毎フレーム、または再描画が必要な時に呼ばれる関数
    pub fn draw(
        &mut self,
        mode: InputMode,
        style: &WindowStyle,
        w: f32,
        h: f32,
        scale: f64,
        transparent: bool,
    ) -> anyhow::Result<()> {
        unsafe {
            // 1作り直し予約があるかチェック
            if let Some(new_mode) = self.pending_alpha_recreation.take() {
                if self
                    .recreate_swapchain(transparent, mode.clone(), style, scale)
                    .is_ok()
                {
                    // 成功したときだけ、現在のモードを書き換える
                    self.current_alpha_mode = new_mode;
                } else {
                    tracing::error!("Failed to recreate swap chain");
                    return Ok(());
                }
            }

            // GPUの準備ができるまで待機
            WaitForSingleObject(self.waitable_object, 1000);
            // SwapChainのバッファをD2Dの描き先に設定
            // 次に書き込むための画用紙（DXGI Surface）を取得
            let dxgi_surface: IDXGISurface = self.swap_chain.GetBuffer(0)?;

            let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: self.current_alpha_mode,
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
            self.d2d_context
                .Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
        }

        // 背景を角丸矩形で描画
        // 極端な縦長を防ぐ
        let right = if w < h { h } else { w };
        let rounded_rect = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right,
                bottom: h,
            },
            radiusX: 2.0, // 角丸の半径
            radiusY: 2.0,
        };

        // paddingを加味した描画領域
        let p = style.padding;
        let text_rect = D2D_RECT_F {
            left: p,
            top: p,
            right: right - p,
            bottom: h - p,
        };

        // 文字列を取得
        // Rustの文字列はUTF-8、WindowsAPIはUTF-16。C言語の名残で最後は0で終わるというルール
        let str = mode.as_str(style.text_format);
        let text: Vec<u16> = str.encode_utf16().chain(std::iter::once(0)).collect();

        // 描画命令
        unsafe {
            self.d2d_context
                .FillRoundedRectangle(&rounded_rect, &self.bg_brush);
            // 中央に描画
            self.d2d_context.DrawText(
                &text,
                &self.format,
                &text_rect,
                &self.font_brush,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        };

        // 描き終えた画用紙を片付けて画面に送信
        unsafe {
            // ここでGPUに描画命令
            self.d2d_context.EndDraw(None, None)?;
            // 描画したバッファを画面に表示
            // matchで、DXGI_STATUS_OCCLUDED（画面が隠れていて描画不要な状態）などの特殊な状況をハンドリングすることも可能
            self.swap_chain.Present(1, DXGI_PRESENT::default()).ok()?;
            // ターゲットを外す
            // リソースの参照を解放するためと、次のフレームでの不具合を防ぐため
            self.d2d_context.SetTarget(None);
        };

        Ok(())
    }

    pub fn resize(&self, w: u32, h: u32, scale: f64) -> anyhow::Result<()> {
        // サイズがゼロの時はリサイズしない
        if w == 0 || h == 0 {
            return Ok(());
        }

        unsafe {
            // D2DコンテキストがSwapChainのバッファを掴んだままだと ResizeBuffers が失敗する
            self.d2d_context.SetTarget(None);

            // バッファのリサイズ
            let w = if w < h { h } else { w };
            self.swap_chain
                .ResizeBuffers(
                    0, // 0 = 現在のバッファ数(2)を維持
                    w,
                    h,
                    DXGI_FORMAT_UNKNOWN,
                    DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT,
                )
                .context("ResizeBuffers Error")?;

            // Visualのサイズ更新
            self.sprite_visual.SetSize(Vector2 {
                X: w as f32 / scale as f32,
                Y: h as f32 / scale as f32,
            })?;

            // DCompのVisualにバッファを再セットし、Commitする
            connect_swap_chain_to_visual(&self.compositor, &self.sprite_visual, &self.swap_chain)?;

            self.d2d_context.SetTarget(None);
        };

        Ok(())
    }

    pub fn recreate_swapchain(
        &mut self,
        transparent: bool,
        mode: InputMode,
        style: &WindowStyle,
        scale: f64,
    ) -> anyhow::Result<()> {
        tracing::info!("Recreate swapchain");
        unsafe {
            self.d2d_context.SetTarget(None);
            // waitable_object を閉じる
            if !self.waitable_object.is_invalid() {
                CloseHandle(self.waitable_object)?;
            }

            let metrics = self.calc_metrics(mode, style.text_format)?;

            let lw = metrics.width + style.padding * 2.0;
            let lh = metrics.height + style.padding * 2.0;

            let pw = (lw * scale as f32).ceil() as u32;
            let ph = (lh * scale as f32).ceil() as u32;

            // 新しいスワップチェーンの作成
            let new_swap_chain =
                create_swap_chain(&self.dxgi_factory, &self.d3d_device, pw, ph, transparent)?;
            self.swap_chain = new_swap_chain;
            self.waitable_object = self
                .swap_chain
                .cast::<IDXGISwapChain2>()?
                .GetFrameLatencyWaitableObject();

            // Visual のサイズと中身を更新
            let lw = if lw < lh { lh } else { lw };
            self.sprite_visual.SetSize(Vector2 { X: lw, Y: lh })?;

            connect_swap_chain_to_visual(&self.compositor, &self.sprite_visual, &self.swap_chain)?;

            let dxgi_surface: IDXGISurface = self.swap_chain.GetBuffer(0)?;
            let d2d_bitmap = self
                .d2d_context
                .CreateBitmapFromDxgiSurface(&dxgi_surface, None)?;

            self.d2d_context.SetTarget(&d2d_bitmap);
        }
        Ok(())
    }

    pub fn request_alpha_mode(&mut self, transparent: bool) {
        tracing::info!("Request alpha mode");
        let mode = if transparent {
            D2D1_ALPHA_MODE_PREMULTIPLIED
        } else {
            D2D1_ALPHA_MODE_IGNORE
        };

        // 現在のモードと違う場合だけ、作り直しを予約する
        if self.current_alpha_mode != mode {
            tracing::info!(
                "current: {:?} -> changed: {:?}",
                self.current_alpha_mode,
                mode
            );
            self.pending_alpha_recreation = Some(mode);
        }
    }

    pub fn set_position(&self, x: f32, y: f32) -> anyhow::Result<()> {
        self.sprite_visual
            .SetOffset(Vector3 { X: x, Y: y, Z: 0.0 })?;
        Ok(())
    }

    pub fn set_opacity(&self, opacity: f32) -> anyhow::Result<()> {
        self.sprite_visual.SetOpacity(opacity)?;
        Ok(())
    }

    pub fn fade_in(&self, opacity: f32) -> anyhow::Result<()> {
        let anim = self.compositor.CreateScalarKeyFrameAnimation()?;

        // 0.16秒かけて
        anim.SetDuration(TimeSpan::from(Duration::from_millis(160)))?;
        // 目標値(1.0)に到達
        anim.InsertKeyFrame(1.0, opacity)?;
        // "Opacity" プロパティに対してアニメーションを開始
        self.sprite_visual.StartAnimation(h!("Opacity"), &anim)?;

        Ok(())
    }

    pub fn fade_out(&self) -> anyhow::Result<()> {
        let anim = self.compositor.CreateScalarKeyFrameAnimation()?;

        anim.SetDuration(TimeSpan::from(Duration::from_millis(160)))?;
        anim.InsertKeyFrame(1.0, 0.0)?;
        self.sprite_visual.StartAnimation(h!("Opacity"), &anim)?;

        Ok(())
    }

    pub fn auto_hide(
        &self,
        opacity: f32,
        auto_hide_time: f32,
        is_refresh: bool,
    ) -> anyhow::Result<()> {
        let fade_duration = 0.16f32;
        let total_time = fade_duration + auto_hide_time + fade_duration;

        let anim = self.compositor.CreateScalarKeyFrameAnimation()?;
        anim.SetDuration(TimeSpan::from(Duration::from_secs_f32(total_time)))?;

        // 開始地点
        if !is_refresh {
            anim.InsertKeyFrame(0.0, 0.0)?;
        }
        // フェードイン完了地点 (fade_duration / total_time)
        let p1 = fade_duration / total_time; // 全体の 10% 地点で不透明度Max
        anim.InsertKeyFrame(p1, opacity)?;

        // 表示維持の終了地点 ((fade_duration + auto_hide_time) / total_time)
        let p2 = (fade_duration + auto_hide_time) / total_time; // 全体の 90% 地点まで不透明度を意地
        anim.InsertKeyFrame(p2, opacity)?;

        // フェードアウト完了地点
        anim.InsertKeyFrame(1.0, 0.0)?; // 透明

        self.sprite_visual.StartAnimation(h!("Opacity"), &anim)?;

        Ok(())
    }

    pub fn auto_hide_no_animaition(&self, opacity: f32, auto_hide_time: f32) -> anyhow::Result<()> {
        let total_time = auto_hide_time;

        let anim = self.compositor.CreateScalarKeyFrameAnimation()?;
        anim.SetDuration(TimeSpan::from(Duration::from_secs_f32(total_time)))?;

        // ステップのイージング関数を作成
        // StepCount(1) にすることで時間の最後に値が即座に切り替わる
        let step_easing = self.compositor.CreateStepEasingFunction()?;
        step_easing.SetStepCount(1)?;

        anim.InsertKeyFrame(0.0, opacity)?;

        anim.InsertKeyFrameWithEasingFunction(1.0, 0.0, &step_easing)?;

        self.sprite_visual.StartAnimation(h!("Opacity"), &anim)?;

        Ok(())
    }

    pub fn mouse_expr_start(&self) -> anyhow::Result<()> {
        self.sprite_visual
            .StartAnimation(h!("Offset"), &self.mouse_expr)?;
        Ok(())
    }

    // マウス追従
    pub fn mouse_tracking(&self, tx: i32, ty: i32) -> anyhow::Result<()> {
        // 共有変数の値を更新
        self.property_set.InsertVector3(
            h!("MousePos"),
            Vector3 {
                X: tx as f32,
                Y: ty as f32,
                Z: 0.0,
            },
        )?;
        Ok(())
    }

    // 実際のフォントサイズを計算
    pub fn calc_metrics(
        &self,
        mode: InputMode,
        f: TextFormat,
    ) -> anyhow::Result<DWRITE_TEXT_METRICS> {
        let str = mode.as_str(f);
        let text: Vec<u16> = str.encode_utf16().chain(std::iter::once(0)).collect();

        let mut metrics: DWRITE_TEXT_METRICS = Default::default();

        unsafe {
            // 無限の大きさを指定して、自然な改行位置を測る
            let text_layout =
                self.dw_factory
                    .CreateTextLayout(&text, &self.format, f32::MAX, f32::MAX)?;
            text_layout.GetMetrics(&mut metrics)?;
        };

        tracing::debug!("current text: {:?} current metrics: {:#?}", mode, metrics);

        Ok(metrics)
    }

    pub fn update_config(
        &mut self,
        style: &WindowStyle,
        transparent: bool,
        mode: InputMode,
        scale: f64,
    ) -> anyhow::Result<()> {
        let mut needs_recreation = false;
        // 色の更新
        unsafe {
            self.font_brush.SetColor(&style.font_color);
            self.bg_brush.SetColor(&style.bg_color);
        };
        self.current_bg_color = style.bg_color;

        // フォントサイズが変わった場合のみ、TextFormatを再生成
        if (self.current_font_size - style.font_size).abs() > f32::EPSILON {
            unsafe {
                let format = self.dw_factory.CreateTextFormat(
                    w!("Noto Sans JP"),
                    None,
                    DWRITE_FONT_WEIGHT_MEDIUM,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    style.font_size,
                    w!("ja-jp"),
                )?;
                format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
                format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;

                self.format = format;
                self.current_font_size = style.font_size;
            }

            needs_recreation = true;
        }

        // リソースを再生成
        if needs_recreation {
            self.recreate_swapchain(transparent, mode, style, scale)?;
        }

        Ok(())
    }
}

fn create_graphics_foundation() -> anyhow::Result<(
    ID3D11Device,
    IDXGIDevice,
    IDXGIFactory2,
    ID2D1Factory1,
    ID2D1DeviceContext,
)> {
    unsafe {
        // D3D11 Deviceの作成
        // 全ての基盤となるGPUとの対話窓口
        // D3D11_CREATE_DEVICE_BGRA_SUPPORTがないと、後でDirect2Dを繋げようとした時にエラーで落ちる
        let mut d3d_device = None;
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
        let d3d_device = d3d_device.context("Nothing d3d_device")?;
        let dxgi_device: IDXGIDevice = d3d_device.cast()?;
        // DXGISwapChain(Flip Model)の作成
        // 描画結果を画面に送り出すためのダブルバッファ
        let dxgi_factory: IDXGIFactory2 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;

        // D2D Deviceの作成
        // D3D11の上で動作する、2D描画（ベクターグラフィックス）用のインターフェース
        let d2d_factory: ID2D1Factory1 =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let d2d_device = d2d_factory.CreateDevice(&dxgi_device)?;
        let d2d_context = d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;

        Ok((
            d3d_device,
            dxgi_device,
            dxgi_factory,
            d2d_factory,
            d2d_context,
        ))
    }
}

pub fn init_dispatcher_queue() -> anyhow::Result<DispatcherQueueController> {
    let options = DispatcherQueueOptions {
        dwSize: std::mem::size_of::<DispatcherQueueOptions>() as u32,
        threadType: DQTYPE_THREAD_CURRENT, // 現在のスレッドをQueueスレッドにする
        apartmentType: DQTAT_COM_NONE,     // COMの初期化は別途行う
    };

    unsafe {
        // 現在のスレッドに DispatcherQueue を紐付ける
        let controller = CreateDispatcherQueueController(options)?;
        Ok(controller)
    }
}

fn create_composition_layer(
    hwnd: HWND,
    _: &IDXGIDevice,
) -> Result<(
    Compositor,
    DesktopWindowTarget,
    ContainerVisual,
    SpriteVisual,
)> {
    // Compositor作成
    let compositor = Compositor::new().expect("Faild to create compositor");
    tracing::info!("Create Compositor OK");

    // HWNDへの紐付け (DesktopWindowTarget)
    let interop: ICompositorDesktopInterop = compositor.cast()?;
    let desktop_target = unsafe { interop.CreateDesktopWindowTarget(hwnd, true)? };
    tracing::info!("Set DesktopWindowTarget");

    // VisualTree の構築
    let root_visual = compositor.CreateContainerVisual()?;
    desktop_target.SetRoot(&root_visual)?;
    tracing::info!("Create Visual OK");

    let sprite_visual = compositor.CreateSpriteVisual()?;
    root_visual.Children()?.InsertAtTop(&sprite_visual)?;
    tracing::info!("Create SpriteVisual OK");

    Ok((compositor, desktop_target, root_visual, sprite_visual))
}

fn connect_swap_chain_to_visual(
    compositor: &Compositor,
    sprite_visual: &SpriteVisual,
    swap_chain: &IDXGISwapChain1,
) -> Result<()> {
    unsafe {
        // Interop を使って SwapChain を CompositionSurface に変換
        let interop: ICompositorInterop = compositor.cast()?;
        let surface = interop.CreateCompositionSurfaceForSwapChain(swap_chain)?;

        // Surface を Brush にして Visual に塗る
        let brush = compositor.CreateSurfaceBrushWithSurface(&surface)?;
        sprite_visual.SetBrush(&brush)?;
    }
    Ok(())
}

fn create_typography(
    style: &WindowStyle,
    mode: InputMode,
) -> anyhow::Result<(IDWriteFactory, IDWriteTextFormat, f32, f32)> {
    unsafe {
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
        let str = mode.as_str(style.text_format);
        let text: Vec<u16> = str.encode_utf16().chain(std::iter::once(0)).collect();
        let text_layout = dw_factory.CreateTextLayout(&text, &format, f32::MAX, f32::MAX)?;

        let mut metrics: DWRITE_TEXT_METRICS = Default::default();
        text_layout.GetMetrics(&mut metrics)?;

        // パディングを足して正確なスワップチェーンのサイズを算出
        let lw = metrics.width + style.padding * 2.0;
        let lh = metrics.height + style.padding * 2.0;

        Ok((dw_factory, format, lw, lh))
    }
}

fn create_swap_chain(
    dxgi_factory: &IDXGIFactory2,
    d3d_device: &ID3D11Device,
    width: u32,
    height: u32,
    transparent: bool,
) -> anyhow::Result<IDXGISwapChain1> {
    // 透明度設定の切り替え
    let alpha_mode = if transparent {
        DXGI_ALPHA_MODE_PREMULTIPLIED
    } else {
        DXGI_ALPHA_MODE_IGNORE
    };

    let width = if width < height { height } else { width };
    let swap_chain_desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: width,                       // 画面の幅
        Height: height,                     // 画面の高さ
        Format: DXGI_FORMAT_B8G8R8A8_UNORM, // 色の並び(Blue, Green, Red, Alpha)
        Stereo: BOOL(0),                    // 3D立体視にするか（基本0）
        SampleDesc: DXGI_SAMPLE_DESC {
            // アンチエイリアスの設定
            Count: 1, // 1なら無効（2D描画はD2D側でやるので1）
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT, // このバッファを何に使うか（出力用）
        BufferCount: 2,                               // ダブルバッファ（描画中と表示中の2枚持つ）
        Scaling: DXGI_SCALING_STRETCH, // ウィンドウサイズが変わった時の引き伸ばし設定
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD, // 最新の高速な画面切り替え方式
        AlphaMode: alpha_mode,
        Flags: DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as u32,
    };

    // DirectComposition(およびWinRT Composition)用のスワップチェーンを作成
    let swap_cahin =
        unsafe { dxgi_factory.CreateSwapChainForComposition(d3d_device, &swap_chain_desc, None) }?;

    Ok(swap_cahin)
}

fn create_expression_animation(
    compositor: &Compositor,
    sprite_visual: &SpriteVisual,
) -> anyhow::Result<(CompositionPropertySet, ExpressionAnimation)> {
    // 共有変数バッファ (PropertySet) を作成
    let property_set = compositor
        .CreatePropertySet()
        .expect("Faild to create create PropertySet");

    let property_name = h!("MousePos");
    property_set.InsertVector3(property_name, Vector3::default())?;

    // 式アニメーションを作成
    let expression = compositor
        .CreateExpressionAnimationWithExpression(h!("Source.MousePos"))
        .expect("Faild to create create ExpressionAnimation");

    // "Source" が何を指しているかを定義
    expression
        .SetReferenceParameter(h!("Source"), &property_set)
        .expect("Faild to set ReferenceParameter");

    // Visual の Offset プロパティに式をバインド
    sprite_visual
        .StartAnimation(h!("Offset"), &expression)
        .expect("Faild to set StartAnimation");

    Ok((property_set, expression))
}
