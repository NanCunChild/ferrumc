#[allow(dead_code)]
use bevy_math::Vec3A;

pub const GRAVITY_ACCELERATION: Vec3A = Vec3A::new(0.0, -0.08, 0.0);

pub const TERMINAL_VELOCITY_Y: f64 = -3.92;

/// Fraction of normal gravity that still acts on an entity submerged in a fluid.
///
/// Buoyancy is modelled as a behaviour (see the swim-to-surface system), not a passive force, so a
/// submerged entity is not weightless — it sinks slowly under this reduced gravity until a swim
/// behaviour carries it back up. Mirrors vanilla's reduced in-fluid gravity.
pub const SUBMERGED_GRAVITY_FACTOR: f32 = 1.0 / 16.0;

// const WATER_BUOYANCY: f64 = 0.09;
//
// const WATER_DRAG: f64 = 0.8;
//
// const WATER_VERTICAL_DRAG: f64 = 0.95;
//
// const GROUND_FRICTION: f64 = 0.85;
//
// const AIR_RESISTANCE: f64 = 0.98;
