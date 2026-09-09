use ember_app_shell::{EmberShell, ShellCommand, StatusKind};
use ember_canvas::{CanvasCamera, Vec2 as CanvasVec2};
use ember_core::StableId;
use ember_documents::{DocumentEnvelope, DocumentKind};
use ember_jobs::JobManager;
use ember_project::ProjectManifest;
use ember_session::{RuntimeProcess, SessionBody, SessionCommand};
use macroquad::prelude::*;
use serde_json::json;
use std::backtrace::Backtrace;
use std::io::Write;
use std::path::{Path, PathBuf};

fn window_conf() -> Conf {
    Conf {
        window_title: "Ember".into(),
        window_width: 1440,
        window_height: 900,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    }
}

fn install_panic_log_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let root = project_root();
        let logs = root.join("artifacts/logs/editor");
        let _ = std::fs::create_dir_all(&logs);
        let crash = format!(
            "Ember editor panic\n{info}\n\nBacktrace:\n{}\n",
            Backtrace::force_capture()
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(logs.join("ember_editor_crash.log"))
            .and_then(|mut file| file.write_all(crash.as_bytes()));
        default_hook(info);
    }));
}

fn project_root() -> PathBuf {
    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if current.join("Cargo.toml").is_file() {
        return current;
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct NativeEditor {
    shell: EmberShell,
    camera: CanvasCamera,
    notice: String,
    last_mouse: (f32, f32),
    jobs: JobManager,
    runtime: Option<RuntimeProcess>,
}

impl NativeEditor {
    fn new() -> Result<Self, String> {
        let mut shell = EmberShell::default();
        let args: Vec<String> = std::env::args().collect();
        if let Some(path) = args.get(1).filter(|value| !value.starts_with('-')) {
            shell
                .load_project(Path::new(path))
                .map_err(|error| error.to_string())?;
        } else {
            let project = ProjectManifest::new(
                StableId::new("project", "untitled").map_err(|error| error.to_string())?,
                "Untitled Ember Project",
            );
            shell.editor.active_project = Some(project.project_id.clone());
            shell.project = Some(project);
        }
        let welcome_id = StableId::new("document", "welcome").map_err(|error| error.to_string())?;
        shell.insert_document(
            DocumentEnvelope {
                format_version: 1,
                id: welcome_id,
                kind: DocumentKind::Custom("welcome".into()),
                revision: 0,
                payload: json!({
                    "title": "Ember",
                    "message": "Integrated 2D / 2.5D / 3D game authoring"
                }),
            },
            "Welcome",
            None,
        );
        Ok(Self {
            shell,
            camera: CanvasCamera::new(CanvasVec2 {
                x: screen_width(),
                y: screen_height(),
            }),
            notice: "FND-05 native host active".into(),
            last_mouse: mouse_position(),
            jobs: JobManager::default(),
            runtime: None,
        })
    }

    fn update(&mut self) {
        self.camera.viewport = CanvasVec2 {
            x: screen_width(),
            y: screen_height(),
        };
        let (_, wheel) = mouse_wheel();
        if wheel.abs() > f32::EPSILON {
            let (mx, my) = mouse_position();
            self.camera.zoom_around(
                CanvasVec2 { x: mx, y: my },
                if wheel > 0.0 { 1.1 } else { 0.9 },
            );
        }
        let mouse = mouse_position();
        if is_mouse_button_down(MouseButton::Middle) {
            self.camera.pan(CanvasVec2 {
                x: mouse.0 - self.last_mouse.0,
                y: mouse.1 - self.last_mouse.1,
            });
        }
        self.last_mouse = mouse;
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if ctrl && is_key_pressed(KeyCode::S) {
            match self.shell.dispatch(&ShellCommand::SaveActiveDocument) {
                Ok(message) => self.notice = message,
                Err(error) => self.notice = error,
            }
        }
        if is_key_pressed(KeyCode::F6) {
            self.start_or_play_runtime();
        }
    }

    fn start_or_play_runtime(&mut self) {
        if let Some(runtime) = self.runtime.as_mut() {
            self.notice = match runtime.request(SessionCommand::Play) {
                Ok(response) => match response.body {
                    SessionBody::Response(response) => response.summary,
                    _ => "PIE runtime responded".into(),
                },
                Err(error) => format!("PIE request failed: {error}"),
            };
            return;
        }
        let executable = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                self.notice = format!("PIE cannot resolve editor executable: {error}");
                return;
            }
        };
        let Some(directory) = executable.parent() else {
            self.notice = "PIE cannot resolve build output directory".into();
            return;
        };
        let runtime_name = format!("ember_runtime_host{}", std::env::consts::EXE_SUFFIX);
        let runtime_path = directory.join(runtime_name);
        if !runtime_path.is_file() {
            self.notice = format!("PIE runtime is not built yet: {}", runtime_path.display());
            return;
        }
        match RuntimeProcess::spawn(&runtime_path, &[]) {
            Ok(mut runtime) => {
                self.notice = match runtime.request(SessionCommand::Snapshot) {
                    Ok(response) => match response.body {
                        SessionBody::Response(response) => {
                            format!("PIE connected: {}", response.summary)
                        }
                        _ => "PIE connected".into(),
                    },
                    Err(error) => format!("PIE connected but handshake failed: {error}"),
                };
                self.runtime = Some(runtime);
            }
            Err(error) => self.notice = format!("PIE launch failed: {error}"),
        }
    }

    fn draw(&self) {
        clear_background(Color::from_rgba(22, 24, 29, 255));
        self.draw_top_bar();
        self.draw_left_panel();
        self.draw_right_panel();
        self.draw_tabs();
        self.draw_canvas();
        self.draw_status();
    }

    fn draw_top_bar(&self) {
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            34.0,
            Color::from_rgba(31, 34, 41, 255),
        );
        draw_text(
            "EMBER",
            12.0,
            23.0,
            20.0,
            Color::from_rgba(235, 237, 242, 255),
        );
        let mut x = 98.0;
        for menu in &self.shell.menus {
            draw_text(
                &menu.label,
                x,
                22.0,
                17.0,
                Color::from_rgba(205, 209, 219, 255),
            );
            x += measure_text(&menu.label, None, 17, 1.0).width + 24.0;
        }
        let project = self
            .shell
            .project
            .as_ref()
            .map(|project| project.name.as_str())
            .unwrap_or("No Project");
        let width = measure_text(project, None, 16, 1.0).width;
        draw_text(
            project,
            screen_width() - width - 16.0,
            22.0,
            16.0,
            Color::from_rgba(157, 166, 184, 255),
        );
    }

    fn draw_left_panel(&self) {
        let top = 68.0;
        let width = self.shell.layout.left_panel_width as f32;
        draw_rectangle(
            0.0,
            top,
            width,
            screen_height() - top - 26.0,
            Color::from_rgba(27, 30, 36, 255),
        );
        draw_text(
            "WORKSPACES",
            14.0,
            top + 24.0,
            16.0,
            Color::from_rgba(152, 161, 180, 255),
        );
        let mut y = top + 52.0;
        for id in self.shell.workspaces.ids() {
            let title = self.shell.workspaces.title(id).unwrap_or("Workspace");
            let active = self.shell.layout.active_workspace.as_ref() == Some(id);
            if active {
                draw_rectangle(
                    8.0,
                    y - 18.0,
                    width - 16.0,
                    28.0,
                    Color::from_rgba(49, 57, 72, 255),
                );
            }
            draw_text(title, 16.0, y, 17.0, Color::from_rgba(215, 219, 228, 255));
            y += 32.0;
        }
    }

    fn draw_right_panel(&self) {
        let width = self.shell.layout.right_panel_width as f32;
        let x = screen_width() - width;
        draw_rectangle(
            x,
            68.0,
            width,
            screen_height() - 94.0,
            Color::from_rgba(27, 30, 36, 255),
        );
        draw_text(
            "INSPECTOR",
            x + 14.0,
            92.0,
            16.0,
            Color::from_rgba(152, 161, 180, 255),
        );
        let selection = self
            .shell
            .services
            .selection
            .primary()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "Nothing selected".into());
        draw_text(
            &selection,
            x + 14.0,
            122.0,
            16.0,
            Color::from_rgba(211, 215, 224, 255),
        );
    }

    fn draw_tabs(&self) {
        let left = self.shell.layout.left_panel_width as f32;
        let right = self.shell.layout.right_panel_width as f32;
        draw_rectangle(
            left,
            34.0,
            screen_width() - left - right,
            34.0,
            Color::from_rgba(25, 28, 34, 255),
        );
        let mut x = left + 8.0;
        for tab in &self.shell.tabs {
            let active = self.shell.active_tab.as_ref() == Some(&tab.document_id);
            let label = if tab.dirty {
                format!("{} *", tab.title)
            } else {
                tab.title.clone()
            };
            let width = measure_text(&label, None, 16, 1.0).width + 28.0;
            draw_rectangle(
                x,
                38.0,
                width,
                28.0,
                if active {
                    Color::from_rgba(48, 53, 64, 255)
                } else {
                    Color::from_rgba(32, 35, 42, 255)
                },
            );
            draw_text(
                &label,
                x + 12.0,
                58.0,
                16.0,
                Color::from_rgba(220, 224, 232, 255),
            );
            x += width + 2.0;
        }
    }

    fn draw_canvas(&self) {
        let left = self.shell.layout.left_panel_width as f32;
        let right = self.shell.layout.right_panel_width as f32;
        let top = 68.0;
        let bottom = 26.0;
        let width = screen_width() - left - right;
        let height = screen_height() - top - bottom;
        draw_rectangle(left, top, width, height, Color::from_rgba(19, 21, 26, 255));
        let spacing = (32.0 * self.camera.zoom).clamp(12.0, 96.0);
        let mut x = left + (self.camera.center.x * self.camera.zoom).rem_euclid(spacing);
        while x < left + width {
            draw_line(
                x,
                top,
                x,
                top + height,
                1.0,
                Color::from_rgba(35, 39, 47, 255),
            );
            x += spacing;
        }
        let mut y = top + (self.camera.center.y * self.camera.zoom).rem_euclid(spacing);
        while y < top + height {
            draw_line(
                left,
                y,
                left + width,
                y,
                1.0,
                Color::from_rgba(35, 39, 47, 255),
            );
            y += spacing;
        }
        draw_text(
            "Ember Canvas",
            left + 24.0,
            top + 34.0,
            22.0,
            Color::from_rgba(145, 155, 176, 255),
        );
        draw_text(
            "Middle-drag pan  |  Wheel zoom  |  F6 PIE",
            left + 24.0,
            top + 58.0,
            16.0,
            Color::from_rgba(104, 113, 131, 255),
        );
    }

    fn draw_status(&self) {
        let y = screen_height() - 26.0;
        draw_rectangle(
            0.0,
            y,
            screen_width(),
            26.0,
            Color::from_rgba(31, 34, 41, 255),
        );
        let status_color = match self.shell.status.kind {
            StatusKind::Ready => Color::from_rgba(149, 206, 158, 255),
            StatusKind::Working => Color::from_rgba(131, 176, 232, 255),
            StatusKind::Warning => Color::from_rgba(230, 190, 113, 255),
            StatusKind::Error => Color::from_rgba(232, 122, 122, 255),
        };
        let status_text = format!("{}  |  Jobs: {}", self.notice, self.jobs.active().len());
        draw_text(&status_text, 10.0, y + 18.0, 15.0, status_color);
        let zoom = format!("{:.0}%", self.camera.zoom * 100.0);
        let width = measure_text(&zoom, None, 15, 1.0).width;
        draw_text(
            &zoom,
            screen_width() - width - 10.0,
            y + 18.0,
            15.0,
            Color::from_rgba(161, 170, 188, 255),
        );
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    install_panic_log_hook();
    let mut editor = match NativeEditor::new() {
        Ok(editor) => editor,
        Err(error) => {
            eprintln!("Ember Editor failed: {error}");
            return;
        }
    };
    loop {
        editor.update();
        editor.draw();
        next_frame().await;
    }
}
