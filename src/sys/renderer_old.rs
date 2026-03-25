use windows::{
    Win32::{
        Foundation::*,
        Graphics::{
            Direct2D::{
                Common::{D2D_RECT_F, D2D_SIZE_U, D2D1_COLOR_F},
                *,
            },
            DirectWrite::*,
        },
    },
    core::*,
};

use crate::sys::uia::text::InputMode;

pub struct D2DRenderer {
    target: ID2D1HwndRenderTarget,
    brush: ID2D1SolidColorBrush,
    format: IDWriteTextFormat,
}

impl D2DRenderer {
    pub fn new(hwnd: HWND, width: u32, height: u32, scale: f64) -> anyhow::Result<Self> {
        unsafe {
            let d2d_factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dw_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

            let format = dw_factory.CreateTextFormat(
                w!("Noto Sans JP"),
                None,
                DWRITE_FONT_WEIGHT_MEDIUM,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                14.0,
                w!("ja-jp"),
            )?;

            format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;

            let rt_props = D2D1_RENDER_TARGET_PROPERTIES::default();
            let hwnd_rt_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd,
                pixelSize: D2D_SIZE_U { width, height },
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };

            let target = d2d_factory.CreateHwndRenderTarget(&rt_props, &hwnd_rt_props)?;
            let brush = target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.95,
                    g: 0.95,
                    b: 0.95,
                    a: 1.0,
                },
                None,
            )?;

            let dpi = (scale * 96.0) as f32;
            target.SetDpi(dpi, dpi);

            Ok(Self {
                target,
                brush,
                format,
            })
        }
    }

    pub fn resize(&self, width: u32, height: u32) {
        unsafe {
            self.target.Resize(&D2D_SIZE_U { width, height }).ok();
        }
    }

    pub fn draw(&self, input_mode: InputMode, width: u32, height: u32) {
        unsafe {
            self.target.BeginDraw();

            self.target
                .SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);

            self.target.Clear(Some(&D2D1_COLOR_F {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 1.0,
            }));

            let rect = D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: width as f32,
                bottom: height as f32 - 3.0,
            };

            let text: Vec<u16> = input_mode.as_str().encode_utf16().collect();

            self.target.DrawText(
                &text,
                &self.format,
                &rect,
                &self.brush,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            self.target.Flush(None, None).unwrap();
            let _ = self.target.EndDraw(None, None);
        }
    }
}
