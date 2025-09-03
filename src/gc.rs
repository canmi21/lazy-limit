/* src/gc.rs */

use crate::config::LimiterConfig;
use crate::types::RequestRecord;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration as TokioDuration, interval};

pub struct GarbageCollector {
    records: Arc<RwLock<HashMap<String, HashMap<String, RequestRecord>>>>,
    config: LimiterConfig,
}

impl GarbageCollector {
    pub fn new(
        records: Arc<RwLock<HashMap<String, HashMap<String, RequestRecord>>>>,
        config: LimiterConfig,
    ) -> Self {
        Self { records, config }
    }

    pub async fn start(self) {
        let mut interval_timer = interval(TokioDuration::from_secs(self.config.gc_interval));

        loop {
            interval_timer.tick().await;
            self.collect_garbage().await;
        }
    }

    async fn collect_garbage(&self) {
        let mut records = self.records.write().await;
        let current_memory = self.estimate_memory_usage(&records);

        if current_memory > self.config.max_memory {
            self.aggressive_cleanup(&mut records).await;
        } else {
            self.routine_cleanup(&mut records).await;
        }
    }

    async fn routine_cleanup(&self, records: &mut HashMap<String, HashMap<String, RequestRecord>>) {
        let max_age = self.config.max_interval().as_seconds() + 300; // Add 5 min buffer

        records.retain(|_who, route_records| {
            route_records.retain(|_route, record| !record.should_cleanup(max_age));
            !route_records.is_empty()
        });
    }

    async fn aggressive_cleanup(
        &self,
        records: &mut HashMap<String, HashMap<String, RequestRecord>>,
    ) {
        self.routine_cleanup(records).await;

        let current_memory = self.estimate_memory_usage(records);
        if current_memory > self.config.max_memory {
            let target_memory = self.config.max_memory * 80 / 100;
            self.remove_oldest_entries(records, target_memory).await;
        }
    }

    async fn remove_oldest_entries(
        &self,
        records: &mut HashMap<String, HashMap<String, RequestRecord>>,
        target_memory: usize,
    ) {
        let mut entries: Vec<(String, String, u64)> = Vec::new();

        for (who, route_records) in records.iter() {
            for (route, record) in route_records.iter() {
                let oldest_time = if record.timestamps.is_empty() {
                    record.window_start
                } else {
                    *record.timestamps.first().unwrap_or(&record.window_start)
                };
                entries.push((who.clone(), route.clone(), oldest_time));
            }
        }

        entries.sort_by_key(|&(_, _, timestamp)| timestamp);

        let mut current_memory = self.estimate_memory_usage(records);
        for (who, route, _) in entries {
            if current_memory <= target_memory {
                break;
            }

            if let Some(route_records) = records.get_mut(&who) {
                if let Some(removed_record) = route_records.remove(&route) {
                    current_memory -= route.len() + removed_record.memory_usage();
                }
                if route_records.is_empty() {
                    records.remove(&who);
                    current_memory -=
                        who.len() + std::mem::size_of::<HashMap<String, RequestRecord>>();
                }
            }
        }
    }

    fn estimate_memory_usage(
        &self,
        records: &HashMap<String, HashMap<String, RequestRecord>>,
    ) -> usize {
        let mut total = 0;

        for (who, route_records) in records.iter() {
            total += who.capacity() + std::mem::size_of::<HashMap<String, RequestRecord>>();

            for (route, record) in route_records.iter() {
                total += route.capacity() + record.memory_usage();
            }
        }

        total
    }
}
