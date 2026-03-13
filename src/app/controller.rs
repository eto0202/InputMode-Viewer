use std::time::{Duration, Instant};

use crate::sys::uia::input_mode::*;
use crate::sys::{input::*, win32};
use crate::{sys::direct2d::D2DRenderer, ui::window::MainWindow};
use tray_icon::TrayIcon;
use tray_icon::{
    TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};
use winit::dpi::PhysicalPosition;
use winit::window::{Window, WindowAttributes, WindowId};
use winit::{application::ApplicationHandler, dpi::Size};
use winit::{dpi::Position, event::WindowEvent};
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows};

const ID_QUIT: &str = "Quit";

#[derive(Debug, Clone)]
pub enum Message {
    Cap(InputCapability),
    Mode(InputMode),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ShowState {
    Hidden,
    FadeIn {
        start_at: Instant,
        duration: Duration,
    },
    Visible,
}

pub struct Controller {
    pub tray_icon: Option<TrayIcon>,
    pub proxy_window: Option<Window>,
    pub main_window: Option<MainWindow>,
    pub renderer: Option<D2DRenderer>,

    pub last_cap: InputCapability,
    pub last_mode: InputMode,

    pub is_visible: bool,

    pub show_state: ShowState,
}

impl ApplicationHandler<Message> for Controller {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.proxy_window.is_none() {
            let attr = WindowAttributes::default()
                .with_visible(false)
                .with_active(false)
                .with_skip_taskbar(true)
                .with_decorations(false)
                .with_max_inner_size(Size::Logical(winit::dpi::LogicalSize {
                    width: 0f64,
                    height: 0f64,
                }))
                .with_position(Position::Logical(winit::dpi::LogicalPosition {
                    x: 0f64,
                    y: 0f64,
                }));

            let window = event_loop.create_window(attr).unwrap();
            self.proxy_window = Some(window);

            let main_window = MainWindow::new(event_loop);

            main_window
                .window
                .set_outer_position(PhysicalPosition::new(10000, 10000));

            win32::set_window_opacity(main_window.hwnd(), 0).ok();
            win32::set_click_through(main_window.hwnd()).ok();
            win32::set_always_on_top(main_window.hwnd(), true).ok();

            let renderer = D2DRenderer::new(
                main_window.hwnd(),
                main_window.window.inner_size().width,
                main_window.window.inner_size().height,
                main_window.window.scale_factor(),
            )
            .unwrap();

            self.main_window = Some(main_window);
            self.renderer = Some(renderer);

            let tray_menu = Menu::new();
            let quit_item = MenuItem::with_id(ID_QUIT, "Quit", true, None);
            tray_menu.append(&quit_item).unwrap();

            let tray_icon = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("My System App")
                .build()
                .unwrap();

            self.tray_icon = Some(tray_icon);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let (Some(main_window), Some(renderer)) = (&mut self.main_window, &self.renderer) else {
            return;
        };
        if id != main_window.window.id() {
            return;
        }

        match event {
            WindowEvent::RedrawRequested => {
                let should_show = match self.last_cap {
                    InputCapability::No => false,
                    InputCapability::Yes => self.last_mode != InputMode::Unknown,
                    InputCapability::Unknown => self.last_mode.is_on(),
                };

                let hwnd = main_window.hwnd();
                let width = main_window.window.inner_size().width;
                let height = main_window.window.inner_size().height;

                let target_opacity = 180u8;
                let fade_duration = Duration::from_millis(160);

                match (should_show, self.show_state) {
                    (false, _) => {
                        win32::set_window_opacity(hwnd, 0).ok();
                        self.show_state = ShowState::Hidden;
                        self.is_visible = false;
                    }
                    (true, ShowState::Hidden) => {
                        renderer.draw(self.last_mode, width, height);
                        self.show_state = ShowState::FadeIn {
                            start_at: Instant::now(),
                            duration: fade_duration,
                        };
                        main_window.window.request_redraw();
                    }
                    (true, ShowState::FadeIn { start_at, duration }) => {
                        renderer.draw(self.last_mode, width, height);

                        let elapsed = start_at.elapsed();
                        let progress = (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.0);

                        let current_opacity = (progress * target_opacity as f32) as u8;
                        win32::set_window_opacity(hwnd, current_opacity).ok();

                        if progress < 1.0 {
                            main_window.window.request_redraw();
                        } else {
                            self.show_state = ShowState::Visible;
                        }
                    }
                    (true, ShowState::Visible) => {
                        renderer.draw(self.last_mode, width, height);
                    }
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.as_ref() {
                ID_QUIT => event_loop.exit(),
                _ => {}
            }
        }

        if self.is_visible {
            if let Some(mw) = &mut self.main_window {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
                win32::set_window_position(mw.hwnd()).ok();
                mw.window.request_redraw();
            }
        } else {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, msg: Message) {
        match msg {
            Message::Cap(cap) => {
                self.last_cap = cap;
            }
            Message::Mode(mode) => {
                self.last_mode = mode;
            }
        }

        if !self.is_visible {
            self.is_visible = true;
        }
    }
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            tray_icon: None,
            proxy_window: None,
            main_window: None,
            renderer: None,
            last_cap: InputCapability::Unknown,
            last_mode: InputMode::Unknown,
            is_visible: false,
            show_state: ShowState::Hidden,
        }
    }
}
