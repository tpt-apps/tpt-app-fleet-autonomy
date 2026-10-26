use serde::{Deserialize, Serialize};

use crate::rng::Rng;
use crate::scenario::Obstacle;

/// Configuration for a horizontal lidar scan.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LidarConfig {
    /// Rays spread evenly around the full circle.
    pub rays: usize,
    /// Returns at or beyond this range are reported as `max_range`, in metres.
    pub max_range: f64,
    /// Half-width of the uniform range noise, in metres. Zero gives exact ranges.
    pub noise: f64,
}

/// Scans in the horizontal plane through `origin`, starting at `yaw`.
///
/// Each entry is the distance to the first obstacle along that ray, or `max_range` if there is
/// none within range. Returns outside the valid range are clamped to `[0, max_range]`.
pub fn lidar_scan(
    obstacles: &[Obstacle],
    origin: [f64; 3],
    yaw: f64,
    config: &LidarConfig,
    rng: &mut Rng,
) -> Vec<f64> {
    (0..config.rays)
        .map(|i| {
            let angle = yaw + std::f64::consts::TAU * i as f64 / config.rays as f64;
            let direction = [angle.cos(), angle.sin(), 0.0];
            let distance = obstacles
                .iter()
                .filter_map(|o| ray_box(origin, direction, o))
                .filter(|&t| t <= config.max_range)
                .fold(config.max_range, f64::min);
            let noisy = distance + rng.uniform(config.noise);
            noisy.clamp(0.0, config.max_range)
        })
        .collect()
}

/// Entry distance along a ray to an axis-aligned box, or `None` if the ray misses it or the
/// box is behind the origin. The slab method is used. A zero direction component means the
/// ray is parallel to that slab, so it misses unless the origin lies inside the slab.
fn ray_box(origin: [f64; 3], direction: [f64; 3], obstacle: &Obstacle) -> Option<f64> {
    let mut t_near = f64::NEG_INFINITY;
    let mut t_far = f64::INFINITY;
    for axis in 0..3 {
        let (lo, hi) = (obstacle.min[axis], obstacle.max[axis]);
        if direction[axis].abs() < 1e-12 {
            if origin[axis] < lo || origin[axis] > hi {
                return None;
            }
        } else {
            let t1 = (lo - origin[axis]) / direction[axis];
            let t2 = (hi - origin[axis]) / direction[axis];
            t_near = t_near.max(t1.min(t2));
            t_far = t_far.min(t1.max(t2));
        }
    }
    if t_near <= t_far && t_near >= 0.0 {
        Some(t_near)
    } else {
        None
    }
}
