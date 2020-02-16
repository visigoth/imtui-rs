use imtui;
use imgui::{Context};
use std::time::SystemTime;
use std;
use std::os as os;

fn main() {
    let mut imgui = Context::create();
    imgui.set_ini_filename(None);

    unsafe {
        let mut screen = imtui::sys::root::ImTui_ImplNcurses_Init(false, 60.0, -1.0);
        imtui::sys::root::ImTui_ImplText_Init();

        let now = SystemTime::now();
        let mut nframes = 0 as i32;
        let mut fval = 123.0;
        loop {
            if now.elapsed().unwrap().as_secs() > 10 {
                break;
            }

            imtui::sys::root::ImTui_ImplNcurses_NewFrame();
            imtui::sys::root::ImTui_ImplText_NewFrame();

            imgui::sys::igNewFrame();

            imgui::sys::igSetNextWindowPos(imgui::sys::ImVec2 {x: 4.0, y: 2.0}, imgui::sys::ImGuiCond_Once as i32, imgui::sys::ImVec2 {x: 0.0, y: 0.0});
            imgui::sys::igSetNextWindowSize(imgui::sys::ImVec2{x: 50.0, y: 10.0}, imgui::sys::ImGuiCond_Once as i32);
            let s1 = std::ffi::CString::new("Hello, world!").expect("");
            let mut p_open = false;
            imgui::sys::igBegin(s1.as_ptr(), &mut p_open, 0);
            nframes += 1;

            let s2 = std::ffi::CString::new("NFrames = %d").expect("");
            imgui::sys::igText(s2.as_ptr(), nframes);

            let s3 = std::ffi::CString::new("Mouse Pos : x = %g, y = %g").expect("");
            let imguiIo = *imgui::sys::igGetIO();
            imgui::sys::igText(s3.as_ptr(), imguiIo.MousePos.x as os::raw::c_double, imguiIo.MousePos.y as os::raw::c_double);

            let s4 = std::ffi::CString::new("Time per frame %.3f ms/frame (%.1f FPS)").expect("");
            imgui::sys::igText(s4.as_ptr(), 1000.0 / imguiIo.Framerate as os::raw::c_double, imguiIo.Framerate as os::raw::c_double);

            let s5 = std::ffi::CString::new("Float:").expect("");
            imgui::sys::igText(s5.as_ptr());
            imgui::sys::igSameLine(0.0, -1.0);

            let s6 = std::ffi::CString::new("##float").expect("");
            let s7 = std::ffi::CString::new("%.3f").expect("");
            imgui::sys::igSliderFloat(s6.as_ptr(), &mut fval, 0.0, 10.0, s7.as_ptr(), 1.0);
            imgui::sys::igEnd();

            //imtui::sys::ShowDemoWindow(&demo);

            imgui::sys::igRender();

            imtui::sys::root::ImTui_ImplText_RenderDrawData(imgui::sys::igGetDrawData(), screen);
            imtui::sys::root::ImTui_ImplNcurses_DrawScreen(true);
        }

        imtui::sys::root::ImTui_ImplNcurses_Shutdown();
    }
}
