use crate::{
    describe, operator,
    store::{Asset, Content, Item, Slide, Store},
    Outcome,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use uuid::Uuid;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Display {
    name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale: f64,
}

impl From<tauri::Monitor> for Display {
    fn from(m: tauri::Monitor) -> Self {
        Self {
            name: m.name().cloned().unwrap_or_default(),
            x: m.position().x,
            y: m.position().y,
            width: m.size().width,
            height: m.size().height,
            scale: m.scale_factor(),
        }
    }
}

impl Display {
    #[cfg(not(target_os = "macos"))]
    fn overlaps(&self, other: &Self) -> bool {
        let (x, y, ox, oy) = (self.x as i64, self.y as i64, other.x as i64, other.y as i64);
        x < ox + other.width as i64
            && ox < x + self.width as i64
            && y < oy + other.height as i64
            && oy < y + self.height as i64
    }
}

#[cfg(target_os = "macos")]
fn extended_target(target: &Display, operator: &Display, topology: &[Display]) -> bool {
    use core_graphics::display::CGDisplay;

    // Tao reports each display's geometry at its own backing scale. Comparing
    // those rectangles can produce a false overlap on an extended desktop.
    target != operator
        && topology.len() >= 2
        && CGDisplay::active_displays().is_ok_and(|ids| {
            ids.len() == topology.len()
                && ids.iter().all(|&id| !CGDisplay::new(id).is_in_mirror_set())
        })
}

#[cfg(not(target_os = "macos"))]
fn extended_target(target: &Display, operator: &Display, topology: &[Display]) -> bool {
    !target.overlaps(operator)
        && topology
            .iter()
            .filter(|display| display.overlaps(target))
            .count()
            == 1
}

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Extended,
    Shared,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    Next,
    Previous,
    Black,
    Restore,
    ToggleBlack,
    Clear,
    Close,
    Take,
    NextItem,
    PreviousItem,
}

#[derive(Default)]
pub struct Session {
    generation: u64,
    revision: u64,
    output: Option<Output>,
    audio: Option<Audio>,
    message: String,
    shared_intent: Option<Intent>,
}

type Shared<'a> = State<'a, Mutex<Session>>;

struct Output {
    label: String,
    display: Display,
    topology: Vec<Display>,
    mode: Mode,
    ready: bool,
    suspended: bool,
    opening: Instant,
    seen: u64,
    video_stopped_at: u64,
    live: Option<Live>,
    pending: Option<Candidate>,
    black: bool,
}

impl Output {
    fn apply(&mut self, intent: Intent) {
        match intent {
            Intent::Black => self.black = true,
            Intent::Restore if self.live.is_some() => self.black = false,
            Intent::ToggleBlack if self.live.is_some() => self.black = !self.black,
            Intent::Next | Intent::Previous => {
                if let Some(live) = &mut self.live {
                    if let Content::Slides { slides, .. } = &live.item.content {
                        live.index = if matches!(intent, Intent::Next) {
                            (live.index + 1).min(slides.len() - 1)
                        } else {
                            live.index.saturating_sub(1)
                        };
                    }
                }
            }
            _ => {}
        }
    }
}

struct Live {
    item: Item,
    index: usize,
    token: Uuid,
}

struct Candidate {
    item: Item,
    token: Uuid,
    ready: bool,
    failed: bool,
}

#[derive(Clone, Serialize)]
struct Audio {
    item: Uuid,
    asset: Asset,
    token: Uuid,
}

#[derive(Clone, Serialize)]
struct Video {
    asset: Asset,
    token: Uuid,
}

impl Live {
    fn video(&self) -> Option<Video> {
        match &self.item.content {
            Content::Video { video } => Some(Video {
                asset: video.clone(),
                token: self.token,
            }),
            _ => None,
        }
    }

    fn slide(&self) -> Option<&Slide> {
        match &self.item.content {
            Content::Slides { slides, .. } => slides.get(self.index),
            _ => None,
        }
    }
}

impl Session {
    fn commit_take(&mut self, token: Uuid) -> Outcome<()> {
        let revision = self.revision + 1;
        let output = self
            .output
            .as_mut()
            .filter(|out| out.ready)
            .ok_or("Projection is unavailable.")?;
        if !output.pending.as_ref().is_some_and(|candidate| {
            candidate.token == token && candidate.ready && !candidate.failed
        }) {
            return Err("Prepared content is no longer ready.".into());
        }
        let candidate = output.pending.take().ok_or("No prepared item.")?;
        if output.live.as_ref().and_then(Live::video).is_some() {
            output.video_stopped_at = revision;
        }
        let had_live = output.live.is_some();
        let item = candidate.item;
        let audio = match &item.content {
            Content::Slides { audio, .. } => audio.as_ref(),
            Content::Audio { audio } => Some(audio),
            Content::Video { .. } => None,
        }
        .map(|asset| Audio {
            item: item.id,
            asset: asset.clone(),
            token,
        });
        output.black = false;
        output.live = Some(Live {
            item,
            index: 0,
            token,
        });
        let preserve_audio = !had_live
            && self
                .audio
                .as_ref()
                .zip(audio.as_ref())
                .is_some_and(|(current, next)| {
                    current.item == next.item && current.asset == next.asset
                });
        if !preserve_audio {
            self.audio = audio;
        }
        self.revision = revision;
        self.message.clear();
        Ok(())
    }

    pub fn allows_media(&self, label: &str, group: Uuid, id: Uuid) -> bool {
        let Some(output) = self
            .output
            .as_ref()
            .filter(|output| output.label == label && output.ready)
        else {
            return false;
        };
        let matches = |asset: &Asset| asset.group == group && asset.id == id;
        if let Some(live) = &output.live {
            if matches!(live.slide(), Some(Slide::Image { asset }) if matches(asset))
                || matches!(&live.item.content, Content::Video { video } if matches(video))
            {
                return true;
            }
        }
        matches!(&output.pending, Some(candidate)
            if matches!(&candidate.item.content, Content::Video { video } if matches(video)))
    }
}

fn monitors(window: &WebviewWindow) -> Outcome<Vec<Display>> {
    let mut list: Vec<_> = window
        .available_monitors()
        .map_err(describe)?
        .into_iter()
        .map(Display::from)
        .collect();
    list.sort_by(|a, b| {
        (a.x, a.y, &a.name, a.width, a.height)
            .cmp(&(b.x, b.y, &b.name, b.width, b.height))
            .then_with(|| a.scale.total_cmp(&b.scale))
    });
    Ok(list)
}

fn current(window: &WebviewWindow) -> Option<Display> {
    window.current_monitor().ok().flatten().map(Display::from)
}

fn main_window(app: &AppHandle) -> Outcome<WebviewWindow> {
    app.get_webview_window("main")
        .ok_or_else(|| "Operator window closed.".into())
}

fn retire(app: &AppHandle, session: &mut Session, message: &str) {
    if let Some(output) = session.output.take() {
        if let Some(window) = app.get_webview_window(&output.label) {
            let _ = window.hide();
            let _ = window.destroy();
        }
        if output.mode == Mode::Shared {
            if let Ok(main) = main_window(app) {
                let _ = main.show();
                let _ = main.set_focus();
            }
        }
    }
    session.shared_intent = None;
    session.message = message.into();
    session.revision += 1;
}

fn check(app: &AppHandle, session: &mut Session) {
    let Some(output) = &mut session.output else {
        return;
    };
    let Some(window) = app.get_webview_window(&output.label) else {
        retire(
            app,
            session,
            "Output closed. Confirm a display and Take again.",
        );
        return;
    };
    if output.suspended {
        return;
    }
    let topology_ok = monitors(&window).is_ok_and(|list| list == output.topology);
    let separation_ok = output.mode == Mode::Shared
        || main_window(app)
            .ok()
            .and_then(|main| current(&main))
            .is_some_and(|main| extended_target(&output.display, &main, &output.topology));
    let placement_ok = if output.ready {
        current(&window).as_ref() == Some(&output.display)
            && window.is_fullscreen().unwrap_or(false)
    } else {
        output.opening.elapsed() < Duration::from_secs(8)
    };
    if topology_ok && separation_ok && placement_ok {
        return;
    }
    if output.mode == Mode::Shared && output.ready {
        output.suspended = true;
        output.ready = false;
        if output.live.as_ref().and_then(Live::video).is_some() {
            output.video_stopped_at = session.revision + 1;
        }
        output.live = None;
        output.pending = None;
        output.black = true;
        session.revision += 1;
        session.shared_intent = None;
        session.message = "Presentation suspended. Press Esc to return to Pelita.".into();
    } else {
        retire(
            app,
            session,
            "Projection suspended. Confirm a display and Take again.",
        );
    }
}

#[derive(Serialize)]
pub struct DisplayList {
    displays: Vec<Display>,
    operator: Option<Display>,
    preferred: Option<usize>,
}

#[tauri::command]
pub async fn displays(window: WebviewWindow, store: State<'_, Store>) -> Outcome<DisplayList> {
    operator(&window)?;
    let displays = monitors(&window)?;
    let saved: Option<Display> = fs::read(store.root.join("preferred-display.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let preferred = saved
        .filter(|saved| !saved.name.is_empty())
        .and_then(|saved| {
            let mut matches = displays
                .iter()
                .enumerate()
                .filter(|(_, display)| display.name == saved.name);
            let (index, target) = matches.next()?;
            (matches.next().is_none() && target == &saved).then_some(index)
        });
    Ok(DisplayList {
        displays,
        operator: current(&window),
        preferred,
    })
}

#[tauri::command]
pub async fn confirm(
    window: WebviewWindow,
    app: AppHandle,
    state: Shared<'_>,
    topology: Vec<Display>,
    index: Option<usize>,
    mode: Mode,
) -> Outcome<()> {
    operator(&window)?;
    let mut session = state.lock().map_err(describe)?;
    let actual = monitors(&window)?;
    if actual != topology {
        return Err("Displays changed. Refresh before confirming.".into());
    }
    let operator_display = current(&window).ok_or("Operator display unavailable.")?;
    let target = if mode == Mode::Extended {
        let target = actual
            .get(index.ok_or("Select a projector.")?)
            .ok_or("Select a projector.")?;
        if !extended_target(target, &operator_display, &actual) {
            return Err(
                "Choose a verified extended display separate from the operator window.".into(),
            );
        }
        target.clone()
    } else {
        operator_display
    };

    retire(&app, &mut session, "Opening black output…");
    session.generation += 1;
    let label = format!("projector-{}", session.generation);
    session.output = Some(Output {
        label: label.clone(),
        display: target.clone(),
        topology: actual,
        mode,
        ready: false,
        suspended: false,
        opening: Instant::now(),
        seen: 0,
        video_stopped_at: 0,
        live: None,
        pending: None,
        black: true,
    });

    let opened = (|| {
        let builder =
            WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("projector.html".into()))
                .title("Pelita Output")
                .visible(false)
                .decorations(false)
                .focused(mode == Mode::Shared)
                .skip_taskbar(true)
                .background_color(tauri::webview::Color(0, 0, 0, 255));
        #[cfg(target_os = "macos")]
        let builder = builder
            .position(
                f64::from(target.x) / target.scale + 32.0,
                f64::from(target.y) / target.scale + 32.0,
            )
            .inner_size(320.0, 240.0);
        let projector = builder.build().map_err(describe)?;
        #[cfg(not(target_os = "macos"))]
        projector
            .set_position(tauri::PhysicalPosition::new(target.x, target.y))
            .map_err(describe)?;
        if mode == Mode::Shared {
            window.hide().map_err(describe)?;
        }
        projector.set_fullscreen(true).map_err(describe)?;
        projector.show().map_err(describe)?;
        if mode == Mode::Shared {
            projector.set_focus().map_err(describe)?;
            projector.as_ref().set_focus().map_err(describe)?;
        }
        Ok(())
    })();
    if opened.is_err() {
        retire(&app, &mut session, "Could not open projection.");
    }
    opened
}

fn remember_display(store: &Store, output: &Output) {
    let target = &output.display;
    if target.name.is_empty()
        || output
            .topology
            .iter()
            .filter(|display| display.name == target.name)
            .count()
            != 1
    {
        return;
    }
    if let Ok(bytes) = serde_json::to_vec(target) {
        let path = store.root.join("preferred-display.tmp");
        if fs::write(&path, bytes).is_ok() {
            let _ = fs::rename(path, store.root.join("preferred-display.json"));
        }
    }
}

#[derive(Serialize)]
pub struct Frame {
    revision: u64,
    black: bool,
    slide: Option<Slide>,
    video: Option<Video>,
    candidate: Option<Video>,
    shared: bool,
}

#[derive(Deserialize)]
pub struct Feedback {
    seen: u64,
    ready: Option<Uuid>,
    failed: Option<Uuid>,
}

#[tauri::command]
pub async fn projector(
    window: WebviewWindow,
    app: AppHandle,
    state: Shared<'_>,
    store: State<'_, Store>,
    feedback: Feedback,
) -> Outcome<Option<Frame>> {
    let mut session = state.lock().map_err(describe)?;
    check(&app, &mut session);
    let revision = session.revision;
    let Some(output) = session
        .output
        .as_mut()
        .filter(|out| out.label == window.label())
    else {
        return Ok(None);
    };
    let newly_ready = !output.ready
        && !output.suspended
        && current(&window).as_ref() == Some(&output.display)
        && window.is_fullscreen().unwrap_or(false);
    if newly_ready {
        output.ready = true;
        if output.mode == Mode::Extended {
            remember_display(&store, output);
        }
    }
    output.seen = output.seen.max(feedback.seen.min(revision));
    if let Some(candidate) = &mut output.pending {
        candidate.ready |= feedback.ready == Some(candidate.token);
        candidate.failed |= feedback.failed == Some(candidate.token);
    }
    if output
        .live
        .as_ref()
        .and_then(Live::video)
        .is_some_and(|video| feedback.failed == Some(video.token))
    {
        output.live = None;
        output.black = true;
        output.video_stopped_at = revision + 1;
        session.revision += 1;
        session.message = "Video playback failed. Take another item or retry.".into();
    } else if newly_ready {
        session.message = "Display confirmed. Ready to Take.".into();
    }
    let output = session.output.as_ref().ok_or("Output closed.")?;
    Ok(Some(Frame {
        revision: session.revision,
        black: output.black,
        slide: output.live.as_ref().and_then(Live::slide).cloned(),
        video: output.live.as_ref().and_then(Live::video),
        candidate: output
            .pending
            .as_ref()
            .and_then(|candidate| match &candidate.item.content {
                Content::Video { video } => Some(Video {
                    asset: video.clone(),
                    token: candidate.token,
                }),
                _ => None,
            }),
        shared: output.mode == Mode::Shared,
    }))
}

#[derive(Serialize)]
pub struct LiveStatus {
    id: Uuid,
    title: String,
    kind: &'static str,
    index: usize,
    count: usize,
    slide: Option<Slide>,
}

#[derive(Serialize)]
pub struct Status {
    message: String,
    ready: bool,
    opening: bool,
    active: bool,
    shared: bool,
    shared_intent: Option<&'static str>,
    black: bool,
    live: Option<LiveStatus>,
    audio: Option<Audio>,
    audio_allowed: bool,
    candidate: Option<(Uuid, bool, bool)>,
}

#[tauri::command]
pub async fn status(window: WebviewWindow, app: AppHandle, state: Shared<'_>) -> Outcome<Status> {
    operator(&window)?;
    let mut session = state.lock().map_err(describe)?;
    check(&app, &mut session);
    let intent = session.shared_intent.take().map(|intent| match intent {
        Intent::NextItem => "next_item",
        Intent::PreviousItem => "previous_item",
        _ => "take",
    });
    let output = session.output.as_ref();
    let live = output.and_then(|output| output.live.as_ref()).map(|live| {
        let (kind, count) = match &live.item.content {
            Content::Slides { slides, .. } => ("slides", slides.len()),
            Content::Video { .. } => ("video", 0),
            Content::Audio { .. } => ("audio", 0),
        };
        LiveStatus {
            id: live.item.id,
            title: live.item.title.clone(),
            kind,
            index: live.index,
            count,
            slide: live.slide().cloned(),
        }
    });
    Ok(Status {
        message: session.message.clone(),
        ready: output.is_some_and(|output| output.ready),
        opening: output.is_some_and(|output| !output.ready && !output.suspended),
        active: output.is_some(),
        shared: output.is_some_and(|output| output.mode == Mode::Shared),
        shared_intent: intent,
        black: output.is_none_or(|output| output.black),
        live,
        audio: session.audio.clone(),
        audio_allowed: session.audio.is_some()
            && output.is_none_or(|output| output.seen >= output.video_stopped_at),
        candidate: output
            .and_then(|output| output.pending.as_ref())
            .map(|candidate| (candidate.token, candidate.ready, candidate.failed)),
    })
}

#[tauri::command]
pub async fn prepare_take(
    window: WebviewWindow,
    app: AppHandle,
    state: Shared<'_>,
    store: State<'_, Store>,
    id: Uuid,
) -> Outcome<Uuid> {
    operator(&window)?;
    let item = store.library.lock().map_err(describe)?.item(id)?;
    for asset in item.assets() {
        store
            .asset(asset)
            .map_err(|_| format!("{} is unavailable. Relink or skip this item.", asset.name))?;
    }
    let mut session = state.lock().map_err(describe)?;
    check(&app, &mut session);
    let output = session
        .output
        .as_mut()
        .filter(|out| out.ready)
        .ok_or("Confirm a ready display first.")?;
    let token = Uuid::new_v4();
    output.pending = Some(Candidate {
        ready: !matches!(&item.content, Content::Video { .. }),
        item,
        token,
        failed: false,
    });
    Ok(token)
}

#[tauri::command]
pub async fn cancel_take(window: WebviewWindow, state: Shared<'_>, token: Uuid) -> Outcome<()> {
    operator(&window)?;
    let mut session = state.lock().map_err(describe)?;
    if let Some(output) = &mut session.output {
        if output
            .pending
            .as_ref()
            .is_some_and(|candidate| candidate.token == token)
        {
            output.pending = None;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn take(
    window: WebviewWindow,
    app: AppHandle,
    state: Shared<'_>,
    token: Uuid,
) -> Outcome<()> {
    operator(&window)?;
    let mut session = state.lock().map_err(describe)?;
    check(&app, &mut session);
    session.commit_take(token)
}

#[tauri::command]
pub async fn action(
    window: WebviewWindow,
    app: AppHandle,
    state: Shared<'_>,
    intent: Intent,
) -> Outcome<()> {
    let mut session = state.lock().map_err(describe)?;
    check(&app, &mut session);
    let operator = window.label() == "main";
    let shared = session
        .output
        .as_ref()
        .is_some_and(|out| out.mode == Mode::Shared && out.label == window.label());
    if !operator && !shared {
        return Err("Output controls unavailable.".into());
    }
    if matches!(intent, Intent::Close) {
        retire(
            &app,
            &mut session,
            "Output closed. Confirm a display to project.",
        );
        return Ok(());
    }
    if matches!(intent, Intent::Clear) {
        session.shared_intent = None;
        let revision = session.revision + 1;
        if let Some(output) = &mut session.output {
            if output.live.as_ref().and_then(Live::video).is_some() {
                output.video_stopped_at = revision;
            }
            output.live = None;
            output.pending = None;
            output.black = true;
        }
        session.audio = None;
        session.revision += 1;
        return Ok(());
    }
    let output = session
        .output
        .as_mut()
        .filter(|out| out.ready)
        .ok_or("Confirm a ready display first.")?;
    match intent {
        Intent::Take | Intent::NextItem | Intent::PreviousItem if shared => {
            session.shared_intent = Some(intent);
            return Ok(());
        }
        Intent::Next | Intent::Previous | Intent::Black | Intent::Restore | Intent::ToggleBlack => {
            output.apply(intent)
        }
        _ => return Ok(()),
    }
    session.revision += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Kind;

    fn session() -> Session {
        let display = Display {
            name: "Test projector".into(),
            x: 100,
            y: 100,
            width: 1920,
            height: 1080,
            scale: 1.0,
        };
        Session {
            output: Some(Output {
                label: "projector-1".into(),
                topology: vec![display.clone()],
                display,
                mode: Mode::Extended,
                ready: true,
                suspended: false,
                opening: Instant::now(),
                seen: 0,
                video_stopped_at: 0,
                live: None,
                pending: None,
                black: true,
            }),
            ..Session::default()
        }
    }

    fn asset(kind: Kind) -> Asset {
        Asset {
            group: Uuid::new_v4(),
            id: Uuid::new_v4(),
            kind,
            name: format!("fixture.{}", kind.extension()),
            size: 1024,
        }
    }

    fn stage(session: &mut Session, item: Item, ready: bool) -> Uuid {
        let token = Uuid::new_v4();
        session.output.as_mut().unwrap().pending = Some(Candidate {
            item,
            token,
            ready,
            failed: false,
        });
        token
    }

    #[test]
    fn navigation_and_mask_preserve_transport_and_clamp() {
        let mut session = session();
        let item = Item {
            id: Uuid::new_v4(),
            title: "Hymn".into(),
            content: Content::Slides {
                slides: vec![
                    Slide::Text {
                        text: "First".into(),
                    },
                    Slide::Text {
                        text: "Last".into(),
                    },
                ],
                audio: Some(asset(Kind::Mp3)),
            },
        };
        let token = stage(&mut session, item.clone(), true);
        session.commit_take(token).unwrap();
        let output = session.output.as_mut().unwrap();
        output.apply(Intent::Previous);
        assert_eq!(output.live.as_ref().unwrap().index, 0);
        output.apply(Intent::Black);
        output.apply(Intent::Next);
        output.apply(Intent::Next);
        assert_eq!(output.live.as_ref().unwrap().index, 1);
        assert!(output.black);
        output.apply(Intent::Restore);
        assert!(!output.black);
        assert_eq!(session.audio.as_ref().unwrap().token, token);

        output.live = None;
        let recovery = stage(&mut session, item, true);
        session.commit_take(recovery).unwrap();
        assert_eq!(session.audio.unwrap().token, token);
    }

    #[test]
    fn video_requires_current_candidate_and_stops_audio_on_commit() {
        let mut session = session();
        let music = Item {
            id: Uuid::new_v4(),
            title: "Music".into(),
            content: Content::Audio {
                audio: asset(Kind::Mp3),
            },
        };
        let music_token = stage(&mut session, music.clone(), true);
        session.commit_take(music_token).unwrap();

        let video = asset(Kind::Mp4);
        let token = stage(
            &mut session,
            Item {
                id: Uuid::new_v4(),
                title: "Video".into(),
                content: Content::Video {
                    video: video.clone(),
                },
            },
            false,
        );
        assert!(session.allows_media("projector-1", video.group, video.id));
        assert!(!session.allows_media("projector-0", video.group, video.id));
        assert!(session.commit_take(token).is_err());
        assert_eq!(session.audio.as_ref().unwrap().token, music_token);
        session
            .output
            .as_mut()
            .unwrap()
            .pending
            .as_mut()
            .unwrap()
            .ready = true;
        assert!(session.commit_take(music_token).is_err());
        session.commit_take(token).unwrap();
        assert!(session.audio.is_none());

        let recovery = stage(&mut session, music, true);
        session.commit_take(recovery).unwrap();
        let output = session.output.unwrap();
        assert_eq!(output.video_stopped_at, session.revision);
        assert!(output.seen < output.video_stopped_at);
    }
}
