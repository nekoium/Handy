//! Bounded, display-only batch snapshots. No native streaming state.
use std::sync::{Arc, Mutex, MutexGuard};

pub(super) const WINDOW: usize = 8 * 16_000;
const FIRST: usize = 2 * 16_000;
const INTERVAL: usize = 3 * 16_000;

pub(super) struct PreviewStatus {
    active: Mutex<bool>,
}

impl PreviewStatus {
    pub fn active(&self) -> MutexGuard<'_, bool> {
        self.active.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn new() -> Self {
        Self {
            active: Mutex::new(true),
        }
    }

    pub fn disable(&self) {
        *self.active() = false;
    }
}

/// One rolling window, no queued preview jobs. Authoritative audio lives elsewhere.
pub(super) struct PreviewQueue {
    pub status: Arc<PreviewStatus>,
    audio: Vec<f32>,
    samples: usize,
    next: usize,
}

impl PreviewQueue {
    pub fn with_status(status: Arc<PreviewStatus>) -> Self {
        Self {
            status,
            audio: Vec::new(),
            samples: 0,
            next: FIRST,
        }
    }

    /// Caller pushes authoritative chunks first. Only an empty route queue permits preview.
    pub fn push(&mut self, pcm: &[f32], idle: bool) -> Option<Vec<f32>> {
        if !*self.status.active() {
            self.audio.clear();
            return None;
        }
        self.samples += pcm.len();
        if pcm.len() >= WINDOW {
            self.audio.clear();
            self.audio.extend_from_slice(&pcm[pcm.len() - WINDOW..]);
        } else {
            let drop = (self.audio.len() + pcm.len()).saturating_sub(WINDOW);
            self.audio.drain(..drop);
            self.audio.extend_from_slice(pcm);
        }
        if !idle || self.samples < self.next {
            return None;
        }
        self.next = self.samples + INTERVAL;
        Some(self.audio.clone())
    }
}

impl Drop for PreviewQueue {
    fn drop(&mut self) {
        self.status.disable();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_snapshots_and_skips_opportunities_while_authoritative_audio_waits() {
        let mut queue = PreviewQueue::with_status(Arc::new(PreviewStatus::new()));
        for _ in 0..300 {
            assert!(queue.push(&vec![1.0; 16_000], false).is_none());
        }
        assert_eq!(queue.audio.len(), WINDOW);
        assert_eq!(queue.push(&[2.0; 320], true).unwrap().len(), WINDOW);
        assert!(queue.push(&[3.0; 320], true).is_none());
        assert!(queue.push(&vec![0.0; INTERVAL], true).is_some());
        assert!(queue.audio.len() <= WINDOW);
    }

    #[test]
    fn stop_discards_snapshot_and_suppresses_a_held_successful_result() {
        let status = Arc::new(PreviewStatus::new());
        let mut queue = PreviewQueue::with_status(status.clone());
        assert!(queue.push(&vec![0.0; FIRST], true).is_some());
        status.disable();
        // The same guard is held through result emission in the app.
        assert!(!*status.active());
        assert!(queue.push(&[0.0; 320], true).is_none());
        assert!(queue.audio.is_empty());
        let fresh = PreviewStatus::new();
        assert!(*fresh.active());
    }

    #[test]
    fn invalidation_waits_only_for_emission_not_inference() {
        use std::sync::mpsc;
        use std::time::Duration;
        let status = Arc::new(PreviewStatus::new());
        let callback = status.clone();
        let (entered, ready) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let active = callback.active();
            assert!(*active);
            entered.send(()).unwrap();
            released.recv().unwrap();
        });
        ready.recv().unwrap();
        let invalidated = status.clone();
        let (done, stopped) = mpsc::channel();
        let closer = std::thread::spawn(move || {
            invalidated.disable();
            done.send(()).unwrap();
        });
        assert!(stopped.recv_timeout(Duration::from_millis(20)).is_err());
        release.send(()).unwrap();
        stopped.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(!*status.active());
        thread.join().unwrap();
        closer.join().unwrap();
    }
}
