use imgui;
use imgui::internal::{RawCast};
use std;

pub use imtui_sys::root as sys;

pub struct Ncurses {
    screen: *mut sys::ImTui::TScreen,
    is_active: bool,
}

impl Ncurses {
    pub fn init(mouse_support: bool, active_fps: f32, idle_fps: f32) -> Ncurses {
        let screen: *mut sys::ImTui::TScreen;
        unsafe {
            screen = sys::ImTui_ImplNcurses_Init(mouse_support, active_fps, idle_fps);
            sys::ImTui_ImplText_Init();
        }
        Ncurses {
            screen: screen,
            is_active: false,
        }
    }

    pub fn set_active(&mut self) {
        self.is_active = true;
    }

    pub fn set_inactive(&mut self) {
        self.is_active = false;
    }

    pub fn new_frame(&self) -> bool {
        let input_pending: bool;
        unsafe {
            input_pending = sys::ImTui_ImplNcurses_NewFrame();
            sys::ImTui_ImplText_NewFrame();
        }
        input_pending
    }

    pub fn render(&self, draw_data: &imgui::DrawData) {
        unsafe {
            let raw_ptr = draw_data.raw() as *const imgui::sys::ImDrawData as *mut imgui::sys::ImDrawData;
            sys::ImTui_ImplText_RenderDrawData(raw_ptr, self.screen);
            sys::ImTui_ImplNcurses_DrawScreen(self.is_active);
        }
    }
}

impl Drop for Ncurses {
    fn drop(&mut self) {
        unsafe {
            sys::ImTui_ImplText_Shutdown();
            sys::ImTui_ImplNcurses_Shutdown();
        }
        self.screen = std::ptr::null_mut();
        self.is_active = false;
    }
}
