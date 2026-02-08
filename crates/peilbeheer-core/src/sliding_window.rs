use std::collections::VecDeque;

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::gemaal::{TrendDirection, TrendInfo, TrendStrength};

/// Statistieken over een sliding window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowStats {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub sum: f64,
    pub first_value: f64,
    pub last_value: f64,
    pub window_start: Option<DateTime<Utc>>,
    pub window_end: Option<DateTime<Utc>>,
    pub window_duration_minutes: f64,
}

/// Sliding window processor voor trend analyse op timeseries data.
pub struct SlidingWindowProcessor {
    window_minutes: i64,
    window: TimeDelta,
    data_points: VecDeque<(DateTime<Utc>, f64)>,
}

impl SlidingWindowProcessor {
    pub fn new(window_minutes: i64) -> Self {
        Self {
            window_minutes,
            window: TimeDelta::minutes(window_minutes),
            data_points: VecDeque::new(),
        }
    }

    /// Voeg een datapunt toe; oude punten buiten het venster worden verwijderd.
    pub fn add_data_point(&mut self, timestamp: DateTime<Utc>, value: f64) {
        let cutoff = timestamp - self.window;
        while self.data_points.front().is_some_and(|(ts, _)| *ts < cutoff) {
            self.data_points.pop_front();
        }
        self.data_points.push_back((timestamp, value));
    }

    /// Voeg serie data toe (Hydronet formaat met timestamp_ms).
    pub fn add_series_data(&mut self, data: &[(i64, f64)]) {
        for &(timestamp_ms, value) in data {
            if timestamp_ms > 0 {
                if let Some(ts) = DateTime::from_timestamp_millis(timestamp_ms) {
                    self.add_data_point(ts, value);
                }
            }
        }
    }

    /// Bereken statistieken over het huidige window.
    pub fn get_window_stats(&self) -> Option<WindowStats> {
        if self.data_points.len() < 2 {
            return None;
        }

        let values: Vec<f64> = self.data_points.iter().map(|(_, v)| *v).collect();
        let sum: f64 = values.iter().sum();
        let count = values.len();

        let first_ts = self.data_points.front().map(|(ts, _)| *ts);
        let last_ts = self.data_points.back().map(|(ts, _)| *ts);
        let duration_minutes = match (first_ts, last_ts) {
            (Some(a), Some(b)) => (b - a).num_seconds() as f64 / 60.0,
            _ => 0.0,
        };

        Some(WindowStats {
            count,
            min: values.iter().cloned().fold(f64::INFINITY, f64::min),
            max: values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            avg: sum / count as f64,
            sum,
            first_value: values[0],
            last_value: values[count - 1],
            window_start: first_ts,
            window_end: last_ts,
            window_duration_minutes: duration_minutes,
        })
    }

    /// Bereken trend met lineaire regressie.
    pub fn get_trend(&self) -> Option<TrendInfo> {
        if self.data_points.len() < 2 {
            return None;
        }

        let first_ts = self.data_points[0].0;
        let times: Vec<f64> = self
            .data_points
            .iter()
            .map(|(ts, _)| (*ts - first_ts).num_seconds() as f64)
            .collect();
        let values: Vec<f64> = self.data_points.iter().map(|(_, v)| *v).collect();

        let n = times.len() as f64;
        let sum_x: f64 = times.iter().sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = times.iter().zip(values.iter()).map(|(x, y)| x * y).sum();
        let sum_x2: f64 = times.iter().map(|x| x * x).sum();

        let denominator = n * sum_x2 - sum_x * sum_x;
        if denominator == 0.0 {
            return None;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denominator;
        let intercept = (sum_y - slope * sum_x) / n;

        // Trend richting
        let direction = if slope.abs() < 0.001 {
            TrendDirection::Stable
        } else if slope > 0.0 {
            TrendDirection::Increasing
        } else {
            TrendDirection::Decreasing
        };

        // R² betrouwbaarheid
        let y_mean = sum_y / n;
        let ss_tot: f64 = values.iter().map(|v| (v - y_mean).powi(2)).sum();
        let ss_res: f64 = times
            .iter()
            .zip(values.iter())
            .map(|(x, y)| (y - (slope * x + intercept)).powi(2))
            .sum();
        let r_squared = if ss_tot > 0.0 {
            1.0 - (ss_res / ss_tot)
        } else {
            0.0
        };

        let strength = if slope.abs() > 0.01 {
            TrendStrength::Strong
        } else if slope.abs() > 0.001 {
            TrendStrength::Moderate
        } else {
            TrendStrength::Weak
        };

        Some(TrendInfo {
            slope: round_to(slope, 6),
            slope_per_hour: round_to(slope * 3600.0, 3),
            direction,
            r_squared: round_to(r_squared, 3),
            strength,
        })
    }

    pub fn window_minutes(&self) -> i64 {
        self.window_minutes
    }

    pub fn len(&self) -> usize {
        self.data_points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data_points.is_empty()
    }
}

fn round_to(value: f64, decimals: u32) -> f64 {
    let factor = 10f64.powi(decimals as i32);
    (value * factor).round() / factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sliding_window_stats() {
        let mut proc = SlidingWindowProcessor::new(30);
        let base = Utc::now();

        proc.add_data_point(base, 1.0);
        proc.add_data_point(base + TimeDelta::minutes(5), 2.0);
        proc.add_data_point(base + TimeDelta::minutes(10), 3.0);

        let stats = proc.get_window_stats().unwrap();
        assert_eq!(stats.count, 3);
        assert!((stats.avg - 2.0).abs() < 0.001);
        assert!((stats.min - 1.0).abs() < 0.001);
        assert!((stats.max - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_sliding_window_trend_increasing() {
        let mut proc = SlidingWindowProcessor::new(30);
        let base = Utc::now();

        for i in 0..10 {
            proc.add_data_point(base + TimeDelta::minutes(i), i as f64 * 0.1);
        }

        let trend = proc.get_trend().unwrap();
        assert_eq!(trend.direction, TrendDirection::Increasing);
        assert!(trend.r_squared > 0.9);
    }

    #[test]
    fn test_sliding_window_trend_stable() {
        let mut proc = SlidingWindowProcessor::new(30);
        let base = Utc::now();

        for i in 0..10 {
            proc.add_data_point(base + TimeDelta::minutes(i), 5.0);
        }

        let trend = proc.get_trend().unwrap();
        assert_eq!(trend.direction, TrendDirection::Stable);
    }

    #[test]
    fn test_window_eviction() {
        let mut proc = SlidingWindowProcessor::new(10);
        let base = Utc::now();

        proc.add_data_point(base, 1.0);
        proc.add_data_point(base + TimeDelta::minutes(5), 2.0);
        // This point is 15 minutes after base, should evict the first point
        proc.add_data_point(base + TimeDelta::minutes(15), 3.0);

        assert_eq!(proc.len(), 2);
    }
}
