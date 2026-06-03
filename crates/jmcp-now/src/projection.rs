use std::{collections::HashMap, sync::Arc};

use arc_swap::ArcSwap;
use chrono::{DateTime, Utc};
use jmcp_app::{AppResult, AppState};

use crate::{
    contract::{NowSnapshot, Scene},
    reads::NowReads,
};

#[derive(Clone, Debug, PartialEq)]
pub struct CachedNow {
    pub generation: i64,
    pub snapshot: NowSnapshot,
    pub scenes: HashMap<String, Scene>,
    pub reads: NowReads,
}

pub struct NowProjection {
    state: AppState,
    cache: ArcSwap<CachedNow>,
    refresh_lock: std::sync::Mutex<()>,
}

impl NowProjection {
    pub fn new(state: AppState, initial: CachedNow) -> Self {
        Self {
            state,
            cache: ArcSwap::from_pointee(initial),
            refresh_lock: std::sync::Mutex::new(()),
        }
    }

    pub fn load(&self) -> Arc<CachedNow> {
        self.cache.load_full()
    }

    pub fn refresh_if_stale(&self) -> AppResult<Arc<CachedNow>> {
        self.refresh_if_stale_at(Utc::now())
    }

    pub fn refresh_if_stale_at(&self, now: DateTime<Utc>) -> AppResult<Arc<CachedNow>> {
        let watermark = self.state.event_watermark()?;
        if self.cache.load().generation == watermark {
            return Ok(self.load());
        }

        let _guard = self
            .refresh_lock
            .lock()
            .map_err(|err| jmcp_app::AppError::State(format!("now projection lock: {err}")))?;
        if self.cache.load().generation == watermark {
            return Ok(self.load());
        }

        let reads = NowReads::from_state(&self.state)?;
        let rebuilt = CachedNow::build(watermark, now, reads);
        self.cache.store(Arc::new(rebuilt));
        Ok(self.load())
    }
}

impl CachedNow {
    pub fn build(generation: i64, captured_at: DateTime<Utc>, reads: NowReads) -> Self {
        let mut scenes = HashMap::new();
        let queue = crate::scenes::queue_blockers::compose(&reads, generation, captured_at);
        scenes.insert(queue.key.clone(), queue.clone());
        let snapshot = crate::scenes::queue_blockers::snapshot(&queue, generation, captured_at);
        Self {
            generation,
            snapshot,
            scenes,
            reads,
        }
    }
}
