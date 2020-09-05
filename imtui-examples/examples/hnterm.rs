extern crate variant_count;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate maplit;

use imtui;
use imgui;
use variant_count::VariantCount;
use std::collections::HashMap;
use std::vec::Vec;

mod hn;

#[derive(VariantCount, PartialEq, Eq, Hash, Clone)]
enum WindowContent {
    Top,
    Show,
    Ask,
    New,
}

lazy_static! {
    static ref CONTENT_TITLE_MAP: HashMap<WindowContent, &'static str> = {
        hashmap! {
            WindowContent::Top => "Top",
            WindowContent::Show => "Show",
            WindowContent::Ask => "Ask",
            WindowContent::New => "New",
        }
    };
}

#[derive(VariantCount)]
enum StoryListMode {
    Micro,
    Normal,
    Spread,
}

struct WindowData {
    window_content: WindowContent,
    show_comments: bool,
    id: hn::ItemId,
    kids: hn::ItemIds,
    score: u32,
    time: u64,
    text: String,
    title: String,
    url: String,
    domain: String,
}

impl WindowData {
    fn new(window_content: WindowContent) -> WindowData {
        WindowData {
            window_content: window_content,
            show_comments: false,
            id: 0,
            kids: vec![],
            score: 0,
            time: 0,
            text: String::from(""),
            title: String::from("[Y] Hacker News"),
            url: String::from(""),
            domain: String::from(""),
        }
    }

    fn render(&mut self, draw_context: &DrawContext, pos: &(f32, f32), size: &(f32, f32)) {
        let title = imgui::ImString::new("[Y] Hacker News");
        let window = imgui::Window::new(&title)
            .position([pos.0, pos.1], imgui::Condition::Always)
            .size([size.0, size.1], imgui::Condition::Always)
            .flags(imgui::WindowFlags::NO_COLLAPSE |
                   imgui::WindowFlags::NO_RESIZE |
                   imgui::WindowFlags::NO_MOVE |
                   imgui::WindowFlags::NO_SCROLLBAR);
        if let Some(window_token) = window.begin(draw_context.ui) {
            // Blank line
            draw_context.ui.text("");

            // Draw a specific story or draw the index
            window_token.end(draw_context.ui);
        }
    }
}

struct DrawContext<'a, 'b> {
    imtui: &'a imtui::Ncurses,
    ui: &'a mut imgui::Ui<'b>,
}

struct AppState {
    windows: Vec<WindowData>
}

struct HntermApp {
    imgui: imgui::Context,
    imtui: imtui::Ncurses,
    state: AppState,
}

impl AppState {
    fn new() -> AppState {
        AppState {
            windows: vec![
                WindowData::new(WindowContent::Top),
            ]
        }
    }

    fn process_input(&mut self, ui: &imgui::Ui) -> bool {
        if ui.is_key_pressed('+' as u32) && self.windows.len() < 3 {
            self.windows.push(WindowData::new(WindowContent::Top))
        }

        !ui.is_key_pressed('q' as u32)
    }
}

impl HntermApp {
    fn new(imgui: imgui::Context, imtui: imtui::Ncurses) -> HntermApp {
        HntermApp {
            imgui,
            imtui,
            state: AppState::new(),
        }
    }

    fn process_frame(&mut self) -> bool {
        self.imtui.set_active();
        self.imtui.new_frame();

        let mut ui = self.imgui.frame();
        if !self.state.process_input(&ui) {
            return false;
        }

        {
            let display_size = ui.io().display_size;
            let window_width = display_size[0] / self.state.windows.len() as f32;
            let window_size = (window_width, display_size[1]);

            let draw_context = DrawContext {
                imtui: &self.imtui,
                ui: &mut ui,
            };

            let mut window_pos = (0.0, 0.0);
            for wd in self.state.windows.iter_mut() {
                wd.render(&draw_context, &window_pos, &window_size);
                window_pos.0 += window_width;
            }
        }

        let draw_data = ui.render();
        self.imtui.render(draw_data);
        true
    }
}

fn set_color_scheme(context: &mut imgui::Context, dark: bool) {
    let light_colors = [
        (imgui::StyleColor::Text, [0.0, 0.0, 0.0, 1.0]),
        (imgui::StyleColor::TextDisabled, [0.6, 0.6, 0.6, 1.0]),
        (imgui::StyleColor::WindowBg, [0.96, 0.96, 0.94, 1.0]),
        (imgui::StyleColor::TitleBg, [1.0, 0.4, 0.0, 1.0]),
        (imgui::StyleColor::TitleBgActive, [1.0, 0.4, 0.0, 1.0]),
        (imgui::StyleColor::TitleBgCollapsed, [0.69, 0.25, 0.0, 1.0]),
        (imgui::StyleColor::ChildBg, [0.96, 0.96, 0.94, 1.0]),
        (imgui::StyleColor::PopupBg, [0.96, 0.96, 0.94, 1.0]),
        (imgui::StyleColor::ModalWindowDimBg, [0.0, 0.0, 0.0, 0.0])
    ];
    let dark_colors = [
        (imgui::StyleColor::Text, [0.0, 1.0, 0.0, 1.0]),
        (imgui::StyleColor::TextDisabled, [0.6, 0.6, 0.6, 1.0]),
        (imgui::StyleColor::WindowBg, [0.0, 0.0, 0.0, 1.0]),
        (imgui::StyleColor::TitleBg, [0.1, 0.2, 0.1, 1.0]),
        (imgui::StyleColor::TitleBgActive, [0.1, 0.2, 0.1, 1.0]),
        (imgui::StyleColor::TitleBgCollapsed, [0.5, 1.0, 0.5, 1.0]),
        (imgui::StyleColor::ChildBg, [0.0, 0.0, 0.0, 1.0]),
        (imgui::StyleColor::PopupBg, [0.0, 0.1, 0.0, 1.0]),
        (imgui::StyleColor::ModalWindowDimBg, [0.0, 0.0, 0.0, 0.0])
    ];
    let colors = if dark { dark_colors } else { light_colors };
    for (style_color, values) in colors.iter() {
        context.style_mut()[*style_color] = *values;
    }
}

fn main() {
    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);
    let imtui = imtui::Ncurses::init(true, 60.0, -1.0);

    set_color_scheme(&mut imgui, false);

    let mut app = HntermApp::new(imgui, imtui);
    while app.process_frame() {}
}
