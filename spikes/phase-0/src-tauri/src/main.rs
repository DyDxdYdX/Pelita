#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Display {
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

#[derive(Clone, Default, Serialize)]
struct Frame {
    revision: u64,
    slide: Option<u8>,
    video: bool,
    black: bool,
}

struct Output {
    label: String,
    display: Display,
    topology: Vec<Display>,
    ready: bool,
    confirmed_at: Instant,
    video_ready: bool,
    frame: Frame,
}

#[derive(Default)]
struct Session {
    generation: u64,
    output: Option<Output>,
    status: String,
}

type Shared<'a> = State<'a, Mutex<Session>>;
type Outcome<T> = Result<T, String>;

fn describe(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn monitors(window: &WebviewWindow) -> Outcome<Vec<Display>> {
    Ok(window
        .available_monitors()
        .map_err(describe)?
        .into_iter()
        .map(Display::from)
        .collect())
}

fn retire(app: &AppHandle, session: &mut Session, reason: &str) {
    if let Some(output) = session.output.take() {
        if let Some(window) = app.get_webview_window(&output.label) {
            let _ = window.hide();
            let _ = window.destroy();
        }
    }
    session.status = reason.into();
}

fn check(app: &AppHandle, session: &mut Session) {
    let Some(output) = &session.output else {
        return;
    };
    let operator_clear = app
        .get_webview_window("main")
        .and_then(|main| main.current_monitor().ok().flatten())
        .is_some_and(|monitor| {
            extended_target(&output.display, &Display::from(monitor), &output.topology)
        });
    let opening = !output.ready && output.confirmed_at.elapsed() < Duration::from_secs(5);
    let valid = operator_clear
        && match app.get_webview_window(&output.label) {
            None => opening,
            Some(window) => {
                monitors(&window).is_ok_and(|topology| topology == output.topology)
                    && (opening
                        || (output.ready
                            && window
                                .current_monitor()
                                .ok()
                                .flatten()
                                .map(Display::from)
                                .as_ref()
                                == Some(&output.display)))
            }
        };
    if !valid {
        retire(
            app,
            session,
            "Projection suspended. Check displays, confirm a target and Take again.",
        );
    }
}

#[tauri::command]
async fn displays(window: WebviewWindow) -> Outcome<Vec<Display>> {
    monitors(&window)
}

#[tauri::command]
async fn confirm(
    window: WebviewWindow,
    app: AppHandle,
    state: Shared<'_>,
    topology: Vec<Display>,
    index: usize,
) -> Outcome<()> {
    if window.label() != "main" {
        return Err("Operator only".into());
    }
    let main = app
        .get_webview_window("main")
        .ok_or("Operator window closed")?;
    let actual = monitors(&main)?;
    if actual != topology {
        return Err("Displays changed. Refresh before confirming.".into());
    }
    let target = actual.get(index).cloned().ok_or("Select a display")?;
    let operator = main
        .current_monitor()
        .map_err(describe)?
        .map(Display::from)
        .ok_or("Operator display unavailable")?;
    if !extended_target(&target, &operator, &actual) {
        return Err(if target == operator {
            "The selected display contains the operator window. Choose the other display."
        } else {
            "Could not verify an extended display. Refresh the display list and check macOS or system display settings."
        }.into());
    }
    let label = {
        let mut session = state.lock().map_err(describe)?;
        retire(&app, &mut session, "Opening black output");
        session.generation += 1;
        let label = format!("projector-{}", session.generation);
        session.output = Some(Output {
            label: label.clone(),
            display: target.clone(),
            topology: actual,
            ready: false,
            confirmed_at: Instant::now(),
            video_ready: false,
            frame: Frame {
                black: true,
                ..Frame::default()
            },
        });
        label
    };
    let result = (|| {
        let builder =
            WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("projector.html".into()))
                .visible(false)
                .decorations(false)
                .focused(false);
        #[cfg(target_os = "macos")]
        let builder = builder
            .position(
                f64::from(target.x) / target.scale + 32.0,
                f64::from(target.y) / target.scale + 32.0,
            )
            .inner_size(320.0, 240.0);
        let window = builder.build().map_err(describe)?;
        #[cfg(not(target_os = "macos"))]
        window
            .set_position(tauri::PhysicalPosition::new(target.x, target.y))
            .map_err(describe)?;
        window.set_fullscreen(true).map_err(describe)?;
        window.show().map_err(describe)
    })();
    let mut session = state.lock().map_err(describe)?;
    if result.is_err()
        || !session
            .output
            .as_ref()
            .is_some_and(|out| out.label == label)
    {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.hide();
            let _ = window.destroy();
        }
        if session
            .output
            .as_ref()
            .is_some_and(|out| out.label == label)
        {
            retire(
                &app,
                &mut session,
                "Could not open projection. Confirm a target again.",
            );
        }
    }
    result
}

#[tauri::command]
async fn projector(
    window: WebviewWindow,
    app: AppHandle,
    state: Shared<'_>,
    video_ready: bool,
    video_failed: bool,
) -> Outcome<Option<Frame>> {
    let mut session = state.lock().map_err(describe)?;
    let Some(output) = session
        .output
        .as_mut()
        .filter(|out| out.label == window.label())
    else {
        return Ok(None);
    };
    let newly_ready = !output.ready;
    if newly_ready {
        if window
            .current_monitor()
            .ok()
            .flatten()
            .map(Display::from)
            .as_ref()
            != Some(&output.display)
        {
            return Ok(None);
        }
        output.ready = true;
    }
    output.video_ready = video_ready;
    if output.frame.video && video_failed {
        output.frame = Frame {
            revision: output.frame.revision + 1,
            black: true,
            ..Frame::default()
        };
        session.status = "Video playback failed. Take slides or reopen output to retry.".into();
    } else if newly_ready {
        session.status = "Display confirmed. Ready to Take.".into();
    }
    check(&app, &mut session);
    Ok(session.output.as_ref().map(|out| out.frame.clone()))
}

#[derive(Serialize)]
struct Status {
    message: String,
    ready: bool,
    video_ready: bool,
    frame: Option<Frame>,
}

#[tauri::command]
async fn status(app: AppHandle, state: Shared<'_>) -> Outcome<Status> {
    let mut session = state.lock().map_err(describe)?;
    check(&app, &mut session);
    let output = session.output.as_ref();
    Ok(Status {
        message: session.status.clone(),
        ready: output.is_some_and(|out| out.ready),
        video_ready: output.is_some_and(|out| out.ready && out.video_ready),
        frame: output.map(|out| out.frame.clone()),
    })
}

#[tauri::command]
async fn action(
    window: WebviewWindow,
    app: AppHandle,
    state: Shared<'_>,
    intent: String,
    slide: u8,
) -> Outcome<()> {
    if window.label() != "main" {
        return Err("Operator only".into());
    }
    let mut session = state.lock().map_err(describe)?;
    check(&app, &mut session);
    let output = session
        .output
        .as_mut()
        .filter(|out| out.ready)
        .ok_or("Confirm a ready display first")?;
    let frame = &mut output.frame;
    match intent.as_str() {
        "take" if slide < 3 => {
            frame.slide = Some(slide);
            frame.video = false;
            frame.black = false;
        }
        "video" if output.video_ready => {
            frame.slide = None;
            frame.video = true;
            frame.black = false;
        }
        "next" => frame.slide = frame.slide.map(|n| (n + 1).min(2)),
        "previous" => frame.slide = frame.slide.map(|n| n.saturating_sub(1)),
        "black" => frame.black = true,
        "restore" if frame.slide.is_some() || frame.video => frame.black = false,
        "restore" => {}
        "clear" => {
            frame.slide = None;
            frame.video = false;
            frame.black = true;
        }
        _ => return Err("Action unavailable".into()),
    }
    frame.revision += 1;
    session.status = "Output confirmed".into();
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(Session::default()))
        .invoke_handler(tauri::generate_handler![
            displays, confirm, projector, status, action
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("start Pelita Phase 0");
}
