//! Non-blocking generation jobs for continuous exterior chunks.

use std::collections::BTreeSet;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use haven_core::SceneMap;

use crate::generated_surface_chunks::{
    generate_surface_chunk_with_profile, generate_surface_pcg_partition_with_profile,
};
use crate::geographic_surface::GeographicGenerationProfile;
use crate::open_world::ChunkCoord;

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceChunkJobRequest {
    pub world_seed: u64,
    pub chunk: ChunkCoord,
    pub profile: GeographicGenerationProfile,
    /// Preserve the initial mainland PCG namespace while expanding beyond the
    /// authored starter rectangle. Generic adventure surfaces leave this None.
    pub pcg_region: Option<String>,
}

#[derive(Debug)]
pub struct SurfaceChunkJobResult {
    pub request: SurfaceChunkJobRequest,
    pub scene: SceneMap,
}

pub struct SurfaceChunkJobQueue {
    request_tx: Sender<SurfaceChunkJobRequest>,
    result_rx: Receiver<SurfaceChunkJobResult>,
    pending: BTreeSet<(i32, i32)>,
}

impl SurfaceChunkJobQueue {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<SurfaceChunkJobRequest>();
        let (result_tx, result_rx) = mpsc::channel::<SurfaceChunkJobResult>();
        thread::Builder::new()
            .name("havenwild-surface-generation".to_string())
            .spawn(move || {
                while let Ok(request) = request_rx.recv() {
                    let scene = if let Some(region) = request.pcg_region.as_deref() {
                        generate_surface_pcg_partition_with_profile(
                            request.world_seed,
                            region,
                            request.chunk,
                            request.profile,
                        )
                    } else {
                        generate_surface_chunk_with_profile(
                            request.world_seed,
                            request.chunk,
                            request.profile,
                        )
                    };
                    if result_tx
                        .send(SurfaceChunkJobResult { request, scene })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("surface generation worker should start");
        Self {
            request_tx,
            result_rx,
            pending: BTreeSet::new(),
        }
    }

    pub fn request(
        &mut self,
        world_seed: u64,
        chunk: ChunkCoord,
        profile: GeographicGenerationProfile,
        pcg_region: Option<&str>,
    ) -> bool {
        let key = (chunk.x, chunk.y);
        if !self.pending.insert(key) {
            return false;
        }
        let request = SurfaceChunkJobRequest {
            world_seed,
            chunk,
            profile,
            pcg_region: pcg_region.map(ToOwned::to_owned),
        };
        if self.request_tx.send(request).is_err() {
            self.pending.remove(&key);
            return false;
        }
        true
    }

    pub fn try_receive(&mut self) -> Option<SurfaceChunkJobResult> {
        match self.result_rx.try_recv() {
            Ok(result) => {
                self.pending
                    .remove(&(result.request.chunk.x, result.request.chunk.y));
                Some(result)
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

impl Default for SurfaceChunkJobQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn duplicate_requests_are_coalesced() {
        let mut queue = SurfaceChunkJobQueue::new();
        let chunk = ChunkCoord::new(4, 5);
        let profile = GeographicGenerationProfile::default();
        assert!(queue.request(42, chunk, profile, None));
        assert!(!queue.request(42, chunk, profile, None));
        let start = Instant::now();
        while queue.try_receive().is_none() && start.elapsed() < Duration::from_secs(2) {
            thread::yield_now();
        }
        assert_eq!(queue.pending_count(), 0);
    }
}
