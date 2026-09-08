use super::*;
use haven_assets::asset_intake::repo_root_dir;
use haven_core::{
    prune_dev_bridge_commands, queue_dev_bridge_command, read_dev_bridge_status, unix_time_ms,
    DevBridgeClientStatus, DevBridgeCommand, DevBridgeCommandEnvelope,
};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const STATUS_POLL_SECONDS: f64 = 0.5;

pub(crate) struct DevClientBridgeState {
    next_sequence: u64,
    auto_push: bool,
    reveal_all_map: bool,
    client_child: Option<Child>,
    build_child: Option<Child>,
    relaunch_after_build: bool,
    last_status: Option<DevBridgeClientStatus>,
    next_status_poll_at: f64,
    last_message: String,
}

impl DevClientBridgeState {
    pub(crate) fn new() -> Self {
        Self {
            next_sequence: unix_time_ms().saturating_mul(1_000),
            auto_push: true,
            reveal_all_map: true,
            client_child: None,
            build_child: None,
            relaunch_after_build: false,
            last_status: None,
            next_status_poll_at: 0.0,
            last_message: "Dev client bridge idle".to_string(),
        }
    }

    pub(crate) fn auto_push(&self) -> bool {
        self.auto_push
    }

    pub(crate) fn toggle_auto_push(&mut self) -> bool {
        self.auto_push = !self.auto_push;
        self.auto_push
    }

    pub(crate) fn connected(&self) -> bool {
        self.last_status
            .as_ref()
            .is_some_and(|status| status.is_fresh(unix_time_ms()))
    }

    pub(crate) fn status_line(&self) -> String {
        let connection = self.last_status.as_ref().and_then(|status| {
            status.is_fresh(unix_time_ms()).then(|| {
                let scene = status.active_scene.as_deref().unwrap_or("frontend");
                let tile = status
                    .player_tile
                    .map(|tile| format!(" @ {},{}", tile[0], tile[1]))
                    .unwrap_or_default();
                format!("Connected | {} | {}{}", status.phase, scene, tile)
            })
        });
        format!(
            "Client {} | Auto Push {} | Map Reveal {} | {}",
            connection.unwrap_or_else(|| "Disconnected".to_string()),
            if self.auto_push { "ON" } else { "OFF" },
            if self.reveal_all_map { "ON" } else { "OFF" },
            self.last_message
        )
    }

    pub(crate) fn poll(&mut self) {
        let now = get_time();
        if now >= self.next_status_poll_at {
            self.next_status_poll_at = now + STATUS_POLL_SECONDS;
            self.refresh_status();
        }
        self.poll_client_child();
        self.poll_build_child();
    }

    pub(crate) fn queue(&mut self, command: DevBridgeCommand) -> Result<u64, String> {
        self.next_sequence = self.next_sequence.saturating_add(1);
        let sequence = self.next_sequence;
        let root = repo_root_dir();
        queue_dev_bridge_command(&root, &DevBridgeCommandEnvelope::new(sequence, command))?;
        self.last_message = format!("Queued command #{sequence}");
        Ok(sequence)
    }

    pub(crate) fn launch_or_attach(&mut self) -> Result<String, String> {
        self.refresh_status();
        if self.connected() {
            self.last_message = "Attached to running client".to_string();
            let _ = self.queue(DevBridgeCommand::SetRevealAllMap {
                enabled: self.reveal_all_map,
            });
            return Ok(self.status_line());
        }
        if self.client_child.as_mut().is_some_and(child_is_running) {
            self.last_message = "Client process is starting".to_string();
            return Ok(self.status_line());
        }
        let root = repo_root_dir();
        let executable = development_client_executable(&root);
        if !executable.is_file() {
            return Err(format!(
                "Development client is not built: {}. Run HavenwildTools.cmd -> Build development (fast).",
                executable.display()
            ));
        }
        let child = Command::new(&executable)
            .current_dir(&root)
            .env("HAVENWILD_ROOT", &root)
            .env("HAVENWILD_DEV_BRIDGE", "1")
            .spawn()
            .map_err(|error| format!("launch {}: {error}", executable.display()))?;
        self.client_child = Some(child);
        self.last_message = "Launched development client; waiting for bridge heartbeat".to_string();
        let _ = self.queue(DevBridgeCommand::SetRevealAllMap {
            enabled: self.reveal_all_map,
        });
        Ok(self.status_line())
    }

    pub(crate) fn request_stop(&mut self) -> Result<String, String> {
        self.queue(DevBridgeCommand::QuitClient)?;
        self.last_message = "Requested client shutdown".to_string();
        Ok(self.status_line())
    }

    pub(crate) fn toggle_reveal_all_map(&mut self) -> Result<bool, String> {
        self.reveal_all_map = !self.reveal_all_map;
        let enabled = self.reveal_all_map;
        self.queue(DevBridgeCommand::SetRevealAllMap { enabled })?;
        Ok(enabled)
    }

    pub(crate) fn build_and_restart(&mut self) -> Result<String, String> {
        if self.build_child.as_mut().is_some_and(child_is_running) {
            return Ok("Development client build is already running".to_string());
        }
        let _ = self.queue(DevBridgeCommand::QuitClient);
        let root = repo_root_dir();
        let script = root.join("tools/build/Build.ps1");
        if !script.is_file() {
            return Err(format!("Build script is missing: {}", script.display()));
        }
        let mut command = Command::new(if cfg!(windows) { "powershell.exe" } else { "pwsh" });
        command
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&script)
            .arg("client")
            .current_dir(&root)
            .env("HAVENWILD_ROOT", &root)
            .stdin(Stdio::null());
        let child = command
            .spawn()
            .map_err(|error| format!("start development client build: {error}"))?;
        self.build_child = Some(child);
        self.relaunch_after_build = true;
        self.last_message = "Building development client in background".to_string();
        Ok(self.status_line())
    }

    fn refresh_status(&mut self) {
        let root = repo_root_dir();
        match read_dev_bridge_status(&root) {
            Ok(Some(status)) => {
                if status.is_fresh(unix_time_ms()) {
                    let _ = prune_dev_bridge_commands(&root, status.last_command_sequence);
                }
                self.last_status = Some(status);
            }
            Ok(None) => self.last_status = None,
            Err(error) => self.last_message = format!("Bridge status read failed: {error}"),
        }
    }

    fn poll_client_child(&mut self) {
        let Some(child) = self.client_child.as_mut() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.last_message = format!("Client process exited ({status})");
                self.client_child = None;
            }
            Ok(None) => {}
            Err(error) => {
                self.last_message = format!("Client process status failed: {error}");
                self.client_child = None;
            }
        }
    }

    fn poll_build_child(&mut self) {
        let Some(child) = self.build_child.as_mut() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.build_child = None;
                if status.success() && self.relaunch_after_build {
                    self.relaunch_after_build = false;
                    self.last_message = "Client build passed; relaunching".to_string();
                    if let Err(error) = self.launch_or_attach() {
                        self.last_message = format!("Build passed but relaunch failed: {error}");
                    }
                } else {
                    self.relaunch_after_build = false;
                    self.last_message = format!("Client build failed ({status})");
                }
            }
            Ok(None) => {}
            Err(error) => {
                self.build_child = None;
                self.relaunch_after_build = false;
                self.last_message = format!("Client build status failed: {error}");
            }
        }
    }
}

impl EditorApp {
    pub(crate) fn poll_dev_client_bridge(&mut self) {
        self.dev_client_bridge.poll();
    }

    pub(crate) fn launch_or_attach_dev_client(&mut self) {
        self.status_message = self
            .dev_client_bridge
            .launch_or_attach()
            .unwrap_or_else(|error| format!("Dev client launch failed: {error}"));
    }

    pub(crate) fn push_assets_to_dev_client(&mut self, reason: &str) {
        self.status_message = match self.dev_client_bridge.queue(DevBridgeCommand::ReloadAssets {
            reason: reason.to_string(),
        }) {
            Ok(sequence) => format!("Pushed asset reload to client #{sequence}"),
            Err(error) => format!("Asset push failed: {error}"),
        };
    }

    pub(crate) fn push_world_to_dev_client(&mut self, reason: &str) {
        self.status_message = match self
            .dev_client_bridge
            .queue(DevBridgeCommand::ReloadEditorWorld {
                reason: reason.to_string(),
            })
        {
            Ok(sequence) => format!("Pushed saved editor world to client #{sequence}"),
            Err(error) => format!("World push failed: {error}"),
        };
    }

    pub(crate) fn save_all_and_push_dev_client(&mut self) {
        let auto_push = self.dev_client_bridge.auto_push();
        self.save_all_editor_documents();
        if self.status_message.starts_with("Save failed:") {
            return;
        }
        if !auto_push {
            self.push_saved_changes_to_dev_client("Save & Push");
        }
    }

    pub(crate) fn auto_push_saved_changes_to_dev_client(&mut self, reason: &str) {
        if self.dev_client_bridge.auto_push() {
            self.push_saved_changes_to_dev_client(reason);
        }
    }

    pub(crate) fn push_saved_changes_to_dev_client(&mut self, reason: &str) {
        self.status_message = match self
            .dev_client_bridge
            .queue(DevBridgeCommand::ReloadAssetsAndWorld {
                reason: reason.to_string(),
            })
        {
            Ok(sequence) => format!("Pushed saved assets + editor world to client #{sequence}"),
            Err(error) => format!("Save & Push failed: {error}"),
        };
    }

    pub(crate) fn build_and_restart_dev_client(&mut self) {
        self.status_message = self
            .dev_client_bridge
            .build_and_restart()
            .unwrap_or_else(|error| format!("Build & Restart failed: {error}"));
    }

    pub(crate) fn stop_dev_client(&mut self) {
        self.status_message = self
            .dev_client_bridge
            .request_stop()
            .unwrap_or_else(|error| format!("Stop client failed: {error}"));
    }

    pub(crate) fn toggle_dev_map_reveal(&mut self) {
        self.status_message = match self.dev_client_bridge.toggle_reveal_all_map() {
            Ok(enabled) => format!(
                "Development full-map reveal {}",
                if enabled { "ON" } else { "OFF" }
            ),
            Err(error) => format!("Development map reveal update failed: {error}"),
        };
    }

    pub(crate) fn toggle_dev_auto_push(&mut self) {
        let enabled = self.dev_client_bridge.toggle_auto_push();
        self.status_message = format!("Dev Client Auto Push {}", if enabled { "ON" } else { "OFF" });
    }

    pub(crate) fn dev_client_status_line(&self) -> String {
        self.dev_client_bridge.status_line()
    }
}

fn development_client_executable(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.join("target/debug/haven_game.exe")
    } else {
        root.join("target/debug/haven_game")
    }
}

fn child_is_running(child: &mut Child) -> bool {
    matches!(child.try_wait(), Ok(None))
}
