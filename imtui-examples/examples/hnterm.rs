extern crate variant_count;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate maplit;

use imtui;
use std::time::{Duration, SystemTime};
use imgui;
use variant_count::VariantCount;
use std::collections::HashMap;
use std::vec::Vec;
use chrono::{DateTime, Utc};
use std::ops::Add;
use reqwest;
use std::error;
use futures::task::LocalSpawnExt;
use futures::executor::LocalPool;
use futures::Future;
use futures::task::Context;
use futures::task::Poll;
use futures::future::poll_fn;
use std::cell::Cell;
use std::rc::Rc;
use core::pin::Pin;

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
    title: String,
    window_content: WindowContent,
    show_comments: bool,
    selected_story_id: Option<HnItemId>,
    hovered_story_id: Option<HnItemId>,
    hovered_comment_id: Option<HnItemId>,
    max_stories: u32,
}

impl WindowData {
    fn new(window_content: WindowContent) -> WindowData {
        WindowData {
            title: String::from("[Y] Hacker News"),
            window_content: window_content,
            show_comments: false,
            selected_story_id: None,
            hovered_story_id: None,
            hovered_comment_id: None,
            max_stories: 10,
        }
    }

    fn render(&self, draw_context: &DrawContext, pos: &(f32, f32), size: &(f32, f32)) {
        let title = imgui::ImString::new(&self.title);
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

type HnItemId = u32;

struct HnStoryItem {
    id: HnItemId,
    by: String,
    score: i32,
    time: DateTime<Utc>,
    text: String,
    title: String,
    url: String,
    domain: String,
    descendants: u32,
    children: Vec<HnItemId>,
}

struct HnCommentItem {
    id: HnItemId,
    by: String,
    score: i32,
    time: DateTime<Utc>,
    text: String,
    children: Vec<HnItemId>,
    parent: HnItemId,
}

struct HnJobItem {
    id: HnItemId,
    by: String,
    score: i32,
    time: DateTime<Utc>,
    title: String,
    url: String,
    domain: String,
}

struct HnPollItem {
    id: HnItemId,
}

struct HnPollOptItem {
    id: HnItemId,
}

enum HnItem {
    Unknown,
    Story { data: HnStoryItem },
    Comment { data: HnCommentItem },
    Job { data: HnJobItem },
    Poll { data: HnPollItem },
    PollOpt { data: HnPollOptItem },
}

struct HnState {
    top_ids: Vec<HnItemId>,
    show_ids: Vec<HnItemId>,
    ask_ids: Vec<HnItemId>,
    new_ids: Vec<HnItemId>,
    changed_ids: Vec<HnItemId>,
}

impl HnState {

    async fn fetch_ids(url: reqwest::Url) -> Result<Vec<HnItemId>, reqwest::Error> {
        reqwest::get(url)
            .await?
            .json::<Vec<HnItemId>>()
            .await
    }

    async fn fetch() -> Result<HnState, Box<dyn error::Error>> {
        let base_url = reqwest::Url::parse("https://hacker-news.firebaseio.com/v0").unwrap();
        let top_ids = HnState::fetch_ids(base_url.join("topstories.json").unwrap()).await?;
        let show_ids = HnState::fetch_ids(base_url.join("showstories.json").unwrap()).await?;
        let ask_ids = HnState::fetch_ids(base_url.join("askstories.json").unwrap()).await?;
        let new_ids = HnState::fetch_ids(base_url.join("newstories.json").unwrap()).await?;
        let changed_ids = HnState::fetch_ids(base_url.join("updates.json").unwrap()).await?;

        let state = HnState {
            top_ids,
            show_ids,
            ask_ids,
            new_ids,
            changed_ids,
        };
        Ok(state)
    }
}

struct AppState {
    windows: Vec<WindowData>,
    hn_state: Rc<Cell<Option<HnState>>>,
    last_update_time: SystemTime,
    next_update: SystemTime,
}

struct HntermApp {
    imgui: imgui::Context,
    imtui: imtui::Ncurses,
    state: AppState,
    executor: LocalPool,
}

impl AppState {
    fn new() -> AppState {
        AppState {
            windows: vec![
                WindowData::new(WindowContent::Top),
                WindowData::new(WindowContent::Top),
                WindowData::new(WindowContent::Top),
            ],
            hn_state: Rc::new(Cell::new(None)),
            last_update_time: SystemTime::UNIX_EPOCH,
            next_update: SystemTime::now(),
        }
    }

    fn process_input(&mut self, ui: &imgui::Ui) -> bool {
        if ui.is_key_pressed('+' as u32) && self.windows.len() < 3 {
            self.windows.push(WindowData::new(WindowContent::Top))
        }

        !ui.is_key_pressed('q' as u32)
    }

    fn update(&mut self, spawner: &impl LocalSpawnExt) {
        let now = SystemTime::now();
        if now.duration_since(self.last_update_time).unwrap() < Duration::new(30, 0) {
            return;
        }

        let state_ref = Rc::clone(&self.hn_state);

        let fetch_and_assign = async move {
            match HnState::fetch().await {
                Ok(state) => state_ref.set(Some(state)),
                _ => ()
            }
            state_ref.set(Some(HnState::fetch().await.unwrap()));
        };
        spawner.spawn_local(fetch_and_assign).unwrap();

        self.last_update_time = now;
        self.next_update = now.add(Duration::new(30, 0));
    }
}

impl HntermApp {
    fn new(imgui: imgui::Context, imtui: imtui::Ncurses) -> HntermApp {
        HntermApp {
            imgui,
            imtui,
            executor: LocalPool::new(),
            state: AppState::new(),
        }
    }

    fn process_frame(&mut self) -> bool {
        self.state.update(&self.executor.spawner());

        for (i, wd) in self.state.windows.iter_mut().enumerate() {
            wd.title = format!(
                "[{}] Hacker News ({})",
                i,
                CONTENT_TITLE_MAP.get(&wd.window_content).unwrap()
            );
        }

        self.imtui.set_active();
        self.imtui.new_frame();

        let mut ui = self.imgui.frame();
        if !self.state.process_input(&ui) {
            return false;
        }

        let draw_context = DrawContext {
            imtui: &self.imtui,
            ui: &mut ui,
        };

        HntermApp::render(&self.state, &draw_context);
        let draw_data = ui.render();
        self.imtui.render(draw_data);
        true
    }

    fn render(state: &AppState, draw_context: &DrawContext) {
        if state.windows.len() == 0 {
            return;
        }

        let display_size = draw_context.ui.io().display_size;
        let windows_to_draw = if display_size[0] < 50.0 {
            &state.windows.as_slice()[0..1]
        } else {
            state.windows.as_slice()
        };

        let window_width = display_size[0] / windows_to_draw.len() as f32;
        let window_size = (window_width, display_size[1]);

        let mut window_pos = (0.0, 0.0);
        let num_windows = windows_to_draw.len();
        for (i, wd) in windows_to_draw.iter().enumerate() {
            let mut actual_window_size = window_size;
            if i != num_windows - 1 {
                actual_window_size.0 = (actual_window_size.0 - 1.1).floor();
            }
            wd.render(draw_context, &window_pos, &actual_window_size);
            window_pos.0 = (window_pos.0 + window_width).ceil();
        }
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);
    let imtui = imtui::Ncurses::init(true, 60.0, -1.0);

    set_color_scheme(&mut imgui, false);

    let mut app = HntermApp::new(imgui, imtui);
    let future_fn = |cx: &mut Context| {
        if app.process_frame() {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    };
    poll_fn(future_fn).await;
    Ok(())
}
