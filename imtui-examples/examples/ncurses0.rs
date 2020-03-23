use imtui;
use imgui;
use std::time::SystemTime;
use std;

fn main() {
    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);

    let mut imtui = imtui::Ncurses::init(true, 60.0, -1.0);
    let now = SystemTime::now();
    let mut nframes = 0 as i32;
    let mut fval = 123.0;

    loop {

        nframes += 1;
        imtui.set_active();
        imtui.new_frame();
        let ui = imgui.frame();
        let title = imgui::ImString::new("Hello, world!");
        let window = imgui::Window::new(&title)
            .position([0.0, 0.0], imgui::Condition::Always)
            .size([50.0, 10.0], imgui::Condition::Always);
        if let Some(windowToken) = window.begin(&ui) {
            ui.text(format!("NFrames = {}", nframes));

            let imgui_io = ui.io();
            ui.text(format!("Mouse Post: x = {}, y = {}", imgui_io.mouse_pos[0], imgui_io.mouse_pos[1]));
            ui.text(format!("Time per frame {0:.3} ms/frame ({1:.1} FPS)", 1000.0 / imgui_io.framerate, imgui_io.framerate));

            ui.text("Float:");
            ui.same_line(0.0);

            let range = std::ops::RangeInclusive::new(0.0, 1000.0);
            let label = imgui::ImString::new("##float");
            let slider_builder = imgui::Slider::new(&label, range);
            slider_builder.build(&ui, &mut fval);
            windowToken.end(&ui);
        }

        let draw_data = ui.render();
        imtui.render(draw_data);
    }
}
