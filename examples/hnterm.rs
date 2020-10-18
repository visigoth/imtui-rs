extern crate variant_count;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate debug_here;

use imtui;
use std::time::Instant;
use std::fmt;
use std::time::{Duration};
use imgui;
use variant_count::VariantCount;
use std::collections::HashMap;
use std::vec::Vec;
use chrono::{DateTime, Utc};
use chrono::naive::NaiveDateTime;
use std::ops::Add;
use reqwest;
use std::error;
use futures::task::LocalSpawnExt;
use futures::executor::LocalPool;
use futures::task::Context;
use futures::task::Poll;
use futures::future::poll_fn;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use clap::Clap;
use eyre::{WrapErr, Result};
use serde::{Deserialize};
use serde_json::Value;

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
    hn_state: Rc<RefCell<HnState>>,
    show_comments: bool,
    selected_story_id: Option<HnItemId>,
    hovered_story_id: Option<HnItemId>,
    hovered_comment_id: Option<HnItemId>,
    max_stories: u32,
}

impl WindowData {
    fn new(window_content: WindowContent, hn_state: &Rc<RefCell<HnState>>) -> WindowData {
        WindowData {
            title: String::from("[Y] Hacker News"),
            window_content: window_content,
            hn_state: Rc::clone(hn_state),
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

#[derive(Deserialize, Clone)]
#[serde(default)]
struct HnStoryItem {
    id: HnItemId,
    by: String,
    score: i32,
    #[serde(with = "chrono::serde::ts_seconds")]
    time: DateTime<Utc>,
    text: String,
    title: String,
    url: String,
    domain: String,
    descendants: u32,
    children: Vec<HnItemId>,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
struct HnCommentItem {
    id: HnItemId,
    by: String,
    score: i32,
    time: DateTime<Utc>,
    text: String,
    children: Vec<HnItemId>,
    parent: HnItemId,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
struct HnJobItem {
    id: HnItemId,
    by: String,
    score: i32,
    time: DateTime<Utc>,
    title: String,
    url: String,
    domain: String,
}

#[derive(Deserialize, Clone)]
struct HnPollItem {
    id: HnItemId,
}

#[derive(Deserialize, Clone)]
struct HnPollOptItem {
    id: HnItemId,
}

#[derive(Clone)]
enum HnItem {
    Unknown,
    Story(HnStoryItem),
    Comment(HnCommentItem),
    Job(HnJobItem),
    Poll(HnPollItem),
    PollOpt(HnPollOptItem),
}

impl Default for HnStoryItem {
    fn default() -> Self {
        HnStoryItem {
            id: 0,
            by: String::from(""),
            score: 0,
            time: DateTime::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
            text: String::from(""),
            title: String::from(""),
            url: String::from(""),
            domain: String::from(""),
            descendants: 0,
            children: vec![],
        }
    }
}

impl Default for HnJobItem {
    fn default() -> Self {
        HnJobItem {
            id: 0,
            by: String::from(""),
            score: 0,
            time: DateTime::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
            title: String::from(""),
            url: String::from(""),
            domain: String::from(""),
        }
    }
}

impl Default for HnCommentItem {
    fn default() -> Self {
        HnCommentItem {
            id: 0,
            by: String::from(""),
            score: 0,
            time: DateTime::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
            text: String::from(""),
            children: vec![],
            parent: 0,
        }
    }
}

impl HnItem {
    fn from_json_value(value: Value) -> Result<HnItem> {
        match &value["type"] {
            Value::String(s) => {
                match s.as_str() {
                    "story" => Ok(HnItem::Story(serde_json::from_value::<HnStoryItem>(value)?)),
                    "job" => Ok(HnItem::Job(serde_json::from_value::<HnJobItem>(value)?)),
                    "comment" => Ok(HnItem::Comment(serde_json::from_value::<HnCommentItem>(value)?)),
                    "poll" => Ok(HnItem::Poll(serde_json::from_value::<HnPollItem>(value)?)),
                    "pollopt" => Ok(HnItem::PollOpt(serde_json::from_value::<HnPollOptItem>(value)?)),
                    // TODO: this should be an error
                    _ => Ok(HnItem::Unknown),
                }
            },
            // TODO: this should be an error
            _ => Ok(HnItem::Unknown)
        }
    }
}

#[derive(Deserialize)]
struct HnUpdatesResponse {
    items: Vec<HnItemId>,
    profiles: Vec<String>,
}

struct HnRefreshResult {
    top_ids: Vec<HnItemId>,
    show_ids: Vec<HnItemId>,
    ask_ids: Vec<HnItemId>,
    new_ids: Vec<HnItemId>,
    changed_ids: HnUpdatesResponse,
}

struct HnState {
    last_url: RefCell<Option<reqwest::Url>>,
    request_bytes: Cell<usize>,
    request_count: Cell<u32>,
    items: Rc<RefCell<HashMap<HnItemId, HnItem>>>,
    items_to_refresh: RefCell<Vec<HnItemId>>,
    last_list_refresh: Option<HnRefreshResult>,
}

impl HnState {

    pub fn new() -> HnState {
        HnState {
            last_url: RefCell::new(None),
            request_bytes: Cell::new(0),
            request_count: Cell::new(0),
            items: Rc::new(RefCell::new(HashMap::new())),
            items_to_refresh: RefCell::new(vec![]),
            last_list_refresh: None,
        }
    }

    async fn fetch_url<T: for<'de> Deserialize<'de>>(&self, url: reqwest::Url) -> Result<T> {
        let url_str = String::from(url.as_str());
        let response = reqwest::get(url.clone())
            .await.wrap_err(format!("Failed to fetch data {}", url_str))?;
        let result = response.text().await?;

        self.last_url.replace(Some(url));
        self.request_bytes.set(self.request_bytes.get() + result.len());
        self.request_count.set(self.request_count.get() + 1);

        serde_json::from_str::<T>(result.as_str()).wrap_err(
            format!("Failed to parse response from {}", url_str)
        )
    }

    async fn fetch(&self) -> Result<HnRefreshResult, Box<dyn error::Error>> {
        let base_url = reqwest::Url::parse("https://hacker-news.firebaseio.com/v0/").unwrap();
        let top_ids = self.fetch_url::<Vec<HnItemId>>(base_url.join("topstories.json").unwrap()).await?;
        let show_ids = self.fetch_url::<Vec<HnItemId>>(base_url.join("showstories.json").unwrap()).await?;
        let ask_ids = self.fetch_url::<Vec<HnItemId>>(base_url.join("askstories.json").unwrap()).await?;
        let new_ids = self.fetch_url::<Vec<HnItemId>>(base_url.join("newstories.json").unwrap()).await?;
        let changed_ids = self.fetch_url::<HnUpdatesResponse>(base_url.join("updates.json").unwrap()).await?;

        let result = HnRefreshResult {
            top_ids,
            show_ids,
            ask_ids,
            new_ids,
            changed_ids,
        };
        Ok(result)
    }

    async fn fetch_item(&self, item_id: u32) -> Result<HnItem> {
        HnItem::from_json_value(self.fetch_item_json(item_id).await?)
    }

    async fn fetch_item_json(&self, item_id: u32) -> Result<Value> {
        let base_url = reqwest::Url::parse(
            "https://hacker-news.firebaseio.com/v0/item/"
        ).unwrap();
        let item_path = format!("{}.json", item_id);
        let url = base_url.join(&item_path).unwrap();
        self.fetch_url::<Value>(url).await
    }
}

#[derive(Debug)]
enum StoryListViewMode {
    Micro,
    Normal,
    Spread,
}

impl fmt::Display for StoryListViewMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result  {
        write!(f, "{:?}", self)
    }
}

const STATUS_WINDOW_HEIGHT: f32 = 4.0;

struct UpdateStatus {
    update_in_progress: bool,
    last_update_time: Option<Instant>,
    next_update: Instant,
}

impl UpdateStatus {
    fn new() -> UpdateStatus {
        UpdateStatus {
            update_in_progress: false,
            last_update_time: None,
            next_update: Instant::now()
        }
    }

    fn update_needed(&self) -> bool {
        if self.update_in_progress {
            return false;
        }

        match self.last_update_time {
            Some(then) => Instant::now().duration_since(then) >= Duration::new(30, 0),
            None => true
        }
    }

    fn set_last_update(&mut self, t: Instant) {
        self.last_update_time = Some(t);
        self.next_update = t.add(Duration::new(30, 0));
    }
}

struct AppState {
    windows: Vec<WindowData>,
    hn_state: Rc<RefCell<HnState>>,
    list_update_status: Rc<RefCell<UpdateStatus>>,
    items_refreshing: bool,
    show_status_window: bool,
    view_mode: StoryListViewMode,
}

impl AppState {
    fn new() -> AppState {
        let hn_state = Rc::new(RefCell::new(HnState::new()));
        AppState {
            windows: vec![
                WindowData::new(WindowContent::Top, &hn_state),
                WindowData::new(WindowContent::Top, &hn_state),
                WindowData::new(WindowContent::Top, &hn_state),
            ],
            hn_state: hn_state,
            list_update_status: Rc::new(RefCell::new(UpdateStatus::new())),
            items_refreshing: false,
            show_status_window: true,
            view_mode: StoryListViewMode::Normal,
        }
    }

    fn process_input(&mut self, ui: &imgui::Ui) -> bool {
        if ui.is_key_pressed('+' as u32) && self.windows.len() < 3 {
            self.windows.push(WindowData::new(WindowContent::Top, &self.hn_state))
        }

        !ui.is_key_pressed('q' as u32)
    }

    fn update(&mut self, spawner: &impl LocalSpawnExt) {
        // Refresh item information as required
        if !self.items_refreshing && self.hn_state.borrow().items_to_refresh.borrow().len() != 0
        {
            self.items_refreshing = true;
            let items_to_fetch = self.hn_state.borrow().items_to_refresh.replace(vec![]);
            let state_ref = Rc::clone(&self.hn_state);
            let fetch_items = async move {
                for item_id in items_to_fetch {
                    let hn_state = state_ref.borrow();
                    let result = hn_state.fetch_item(item_id).await;
                    let mut items_map = hn_state.items.borrow_mut();
                    match result {
                        Ok(item) => {
                            // Need a copy in order to move the value into the map (without Rc).
                            let item_clone = item.clone();
                            match item {
                                HnItem::Story(story) => { items_map.insert(story.id, item_clone); }
                                HnItem::Comment(comment) => { items_map.insert(comment.id, item_clone); }
                                HnItem::Job(job) => { items_map.insert(job.id, item_clone); }
                                HnItem::Poll(poll) => { items_map.insert(poll.id, item_clone); }
                                HnItem::PollOpt(pollopt) => { items_map.insert(pollopt.id, item_clone); }
                                _ => {}
                            }
                        },
                        _ => {}
                    }
                }
            };
            spawner.spawn_local(fetch_items).unwrap();
        }

        // Update the list of items to be shown every 30 seconds
        if self.list_update_status.borrow().update_needed() {
            {
                let mut update_status = self.list_update_status.borrow_mut();
                update_status.update_in_progress = true;
            }

            let state_ref = Rc::clone(&self.hn_state);
            let update_status_ref = Rc::clone(&self.list_update_status);

            let fetch_and_assign = async move {
                let result = {
                    state_ref.borrow().fetch().await
                };
                match result {
                    Ok(result) => {
                        let mut state = state_ref.borrow_mut();
                        // TODO: remove
                        // For now, add all ids to the items_to_refresh list
                        {
                            let mut items_to_refresh = state.items_to_refresh.borrow_mut();
                            items_to_refresh.extend(result.top_ids.iter());
                            items_to_refresh.extend(result.show_ids.iter());
                            items_to_refresh.extend(result.ask_ids.iter());
                            items_to_refresh.extend(result.new_ids.iter());
                            items_to_refresh.extend(result.changed_ids.items.iter());
                        }
                        state.last_list_refresh = Some(result);
                    },
                    _ => ()
                }

                update_status_ref.borrow_mut().set_last_update(Instant::now());
            };
            spawner.spawn_local(fetch_and_assign).unwrap();
        }
    }
}

struct HntermApp {
    imgui: imgui::Context,
    imtui: imtui::Ncurses,
    state: AppState,
    executor: LocalPool,
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
        self.executor.run_until_stalled();

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
        {
            let windows_to_draw = if display_size[0] < 50.0 {
                &state.windows.as_slice()[0..1]
            } else {
                state.windows.as_slice()
            };

            let window_width = display_size[0] / windows_to_draw.len() as f32;
            let mut window_height = display_size[1];
            if state.show_status_window {
                window_height -= STATUS_WINDOW_HEIGHT;
            }

            let window_size = (window_width, window_height);

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

        // Draw the status window
        if state.show_status_window {
            let title = imgui::ImString::new(
                format!("Status | Story List Mode: {}", state.view_mode.to_string())
            );
            let window = imgui::Window::new(&title)
                .position([0., display_size[1] - STATUS_WINDOW_HEIGHT], imgui::Condition::Always)
                .size([display_size[0], STATUS_WINDOW_HEIGHT], imgui::Condition::Always)
                .flags(imgui::WindowFlags::NO_COLLAPSE |
                       imgui::WindowFlags::NO_RESIZE |
                       imgui::WindowFlags::NO_MOVE |
                       imgui::WindowFlags::NO_SCROLLBAR);
            if let Some(window_token) = window.begin(draw_context.ui) {
                let now = Instant::now();
                let time_left = if now >= state.list_update_status.borrow().next_update {
                    Duration::new(0,0)
                } else {
                    state.list_update_status.borrow().next_update.duration_since(now)
                };
                let hn_state = state.hn_state.borrow();
                let opt = hn_state.last_url.borrow();
                let last_url = match opt.as_ref() {
                    Some(r) => String::from(r.as_str()),
                    None => String::from("[None]")
                };

                draw_context.ui.text(
                    format!(" API requests     : {} / {} B (next update in {} s)",
                            hn_state.request_count.get(),
                            hn_state.request_bytes.get(),
                            time_left.as_secs()
                    )
                );
                draw_context.ui.text(
                    format!(" Last API request: {}", last_url)
                );
                window_token.end(draw_context.ui);
            }
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

#[derive(Clap)]
struct Opts {
    #[clap(short, long, about = "Wait for debugger at startup")]
    debug: bool
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let opts = Opts::parse();
    if opts.debug {
        debug_here!();
    }

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
