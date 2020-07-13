extern crate variant_count;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate maplit;

use imtui;
use imgui;
use variant_count::VariantCount;
use std::collections::HashMap;

mod hn;

#[derive(VariantCount, PartialEq, Eq, Hash, Clone)]
enum WindowContent {
    Top,
    Show,
    Ask,
    New,
}

lazy_static! {
    static ref ContentTitle: HashMap<WindowContent, &'static str> = {
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

struct WindowData<'a> {
    window_content: WindowContent,
    show_comments: bool,
    id: hn::ItemId,
    kids: hn::ItemIds,
    score: u32,
    time: u64,
    text: &'a str,
    title: &'a str,
    url: &'a str,
    domain: &'a str,
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

fn render_frame(context: &mut imgui::Context, imtui: &mut imtui::Ncurses) -> bool {
    imtui.set_active();
    imtui.new_frame();

    let ui = context.frame();

    // Only draw 1 window, as opposed to multiple supported by imtui hnterm example
    let title = imgui::ImString::new("[Y] Hacker News");
    let window = imgui::Window::new(&title)
        .position([0.0, 0.0], imgui::Condition::Always)
        .size(ui.io().display_size, imgui::Condition::Always)
        .flags(imgui::WindowFlags::NO_COLLAPSE |
               imgui::WindowFlags::NO_RESIZE |
               imgui::WindowFlags::NO_MOVE |
               imgui::WindowFlags::NO_SCROLLBAR);
    if let Some(window_token) = window.begin(&ui) {
        // Blank line
        ui.text("");

        // Draw a specific story or draw the index
        window_token.end(&ui);
    }
    let draw_data = ui.render();
    imtui.render(draw_data);
    true
}

fn main() {
    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);
    let mut imtui = imtui::Ncurses::init(true, 60.0, -1.0);

    set_color_scheme(&mut imgui, false);
    while render_frame(&mut imgui, &mut imtui) {
    }
}
