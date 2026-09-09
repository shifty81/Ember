use ember_core::StableId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JobSnapshot {
    pub id: StableId,
    pub kind: String,
    pub owner: String,
    pub state: JobState,
    pub progress: f32,
    pub status: String,
    pub cancellable: bool,
    pub parent: Option<StableId>,
    pub artifacts: Vec<PathBuf>,
    pub diagnostics: Vec<String>,
    pub logs: Vec<String>,
    pub started_unix_ms: Option<u128>,
    pub finished_unix_ms: Option<u128>,
}

#[derive(Clone, Debug, Default)]
pub struct JobOutput {
    pub summary: Option<String>,
    pub artifacts: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct JobContext {
    cancelled: Arc<AtomicBool>,
    shared: Arc<Mutex<JobSnapshot>>,
}

impl JobContext {
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn set_progress(&self, progress: f32, status: impl Into<String>) {
        if let Ok(mut snapshot) = self.shared.lock() {
            snapshot.progress = progress.clamp(0.0, 1.0);
            snapshot.status = status.into();
        }
    }

    pub fn add_diagnostic(&self, diagnostic: impl Into<String>) {
        if let Ok(mut snapshot) = self.shared.lock() {
            snapshot.diagnostics.push(diagnostic.into());
        }
    }

    pub fn add_log(&self, line: impl Into<String>) {
        if let Ok(mut snapshot) = self.shared.lock() {
            snapshot.logs.push(line.into());
        }
    }
}

struct JobEntry {
    cancelled: Arc<AtomicBool>,
    shared: Arc<Mutex<JobSnapshot>>,
}

pub struct JobManager {
    next_id: AtomicU64,
    jobs: Arc<Mutex<BTreeMap<StableId, JobEntry>>>,
}

impl Default for JobManager {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            jobs: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

impl JobManager {
    pub fn submit<F>(
        &self,
        kind: impl Into<String>,
        owner: impl Into<String>,
        parent: Option<StableId>,
        work: F,
    ) -> StableId
    where
        F: FnOnce(JobContext) -> Result<JobOutput, String> + Send + 'static,
    {
        let sequence = self.next_id.fetch_add(1, Ordering::Relaxed);
        let id = StableId::new("job", &sequence.to_string()).expect("numeric job id");
        let cancelled = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Mutex::new(JobSnapshot {
            id: id.clone(),
            kind: kind.into(),
            owner: owner.into(),
            state: JobState::Queued,
            progress: 0.0,
            status: "Queued".into(),
            cancellable: true,
            parent,
            artifacts: vec![],
            diagnostics: vec![],
            logs: vec![],
            started_unix_ms: None,
            finished_unix_ms: None,
        }));
        self.jobs.lock().expect("job registry").insert(
            id.clone(),
            JobEntry {
                cancelled: cancelled.clone(),
                shared: shared.clone(),
            },
        );
        let _worker = thread::spawn(move || {
            if let Ok(mut snapshot) = shared.lock() {
                snapshot.state = JobState::Running;
                snapshot.status = "Running".into();
                snapshot.started_unix_ms = Some(unix_ms());
            }
            let context = JobContext {
                cancelled: cancelled.clone(),
                shared: shared.clone(),
            };
            let result = work(context);
            if let Ok(mut snapshot) = shared.lock() {
                if cancelled.load(Ordering::Relaxed) {
                    snapshot.state = JobState::Cancelled;
                    snapshot.status = "Cancelled".into();
                } else {
                    match result {
                        Ok(output) => {
                            snapshot.state = JobState::Succeeded;
                            snapshot.progress = 1.0;
                            snapshot.status = output.summary.unwrap_or("Succeeded".into());
                            snapshot.artifacts.extend(output.artifacts);
                        }
                        Err(error) => {
                            snapshot.state = JobState::Failed;
                            snapshot.status = "Failed".into();
                            snapshot.diagnostics.push(error);
                        }
                    }
                }
                snapshot.finished_unix_ms = Some(unix_ms());
            }
        });
        id
    }

    pub fn cancel(&self, id: &StableId) -> bool {
        let jobs = self.jobs.lock().expect("job registry");
        let Some(entry) = jobs.get(id) else {
            return false;
        };
        entry.cancelled.store(true, Ordering::Relaxed);
        true
    }

    pub fn snapshot(&self, id: &StableId) -> Option<JobSnapshot> {
        let jobs = self.jobs.lock().ok()?;
        let entry = jobs.get(id)?;
        let snapshot = entry.shared.lock().ok()?.clone();
        Some(snapshot)
    }

    pub fn active(&self) -> Vec<JobSnapshot> {
        let Ok(jobs) = self.jobs.lock() else {
            return vec![];
        };
        jobs.values()
            .filter_map(|entry| {
                let snapshot = entry.shared.lock().ok()?;
                Some((*snapshot).clone())
            })
            .filter(|snapshot| matches!(snapshot.state, JobState::Queued | JobState::Running))
            .collect()
    }

    pub fn wait(&self, id: &StableId, timeout: Duration) -> Option<JobSnapshot> {
        let started = std::time::Instant::now();
        loop {
            let snapshot = self.snapshot(id)?;
            if !matches!(snapshot.state, JobState::Queued | JobState::Running) {
                return Some(snapshot);
            }
            if started.elapsed() >= timeout {
                return Some(snapshot);
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProcessSpec {
    pub program: String,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub environment: BTreeMap<String, String>,
}

impl JobManager {
    pub fn submit_process(
        &self,
        kind: impl Into<String>,
        owner: impl Into<String>,
        parent: Option<StableId>,
        spec: ProcessSpec,
    ) -> StableId {
        self.submit(kind, owner, parent, move |context| {
            if context.is_cancelled() {
                return Err("process job cancelled before launch".into());
            }
            context.set_progress(0.05, format!("Starting {}", spec.program));
            let mut command = ProcessCommand::new(&spec.program);
            command.args(&spec.args);
            if let Some(directory) = &spec.working_directory {
                command.current_dir(directory);
            }
            command.envs(&spec.environment);
            let output = command.output().map_err(|error| error.to_string())?;
            if !output.stdout.is_empty() {
                context.add_log(String::from_utf8_lossy(&output.stdout).to_string());
            }
            if !output.stderr.is_empty() {
                context.add_log(String::from_utf8_lossy(&output.stderr).to_string());
            }
            if context.is_cancelled() {
                return Err("process job cancelled".into());
            }
            if !output.status.success() {
                return Err(format!(
                    "process {} exited with {}",
                    spec.program,
                    output.status.code().unwrap_or(-1)
                ));
            }
            context.set_progress(1.0, "Process complete");
            Ok(JobOutput {
                summary: Some(format!("{} completed", spec.program)),
                artifacts: vec![],
            })
        })
    }
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_reports_success_and_progress() {
        let manager = JobManager::default();
        let id = manager.submit("test", "unit", None, |context| {
            context.set_progress(0.5, "Halfway");
            Ok(JobOutput {
                summary: Some("Done".into()),
                artifacts: vec![],
            })
        });
        let snapshot = manager.wait(&id, Duration::from_secs(1)).unwrap();
        assert_eq!(snapshot.state, JobState::Succeeded);
        assert_eq!(snapshot.progress, 1.0);
    }
}
