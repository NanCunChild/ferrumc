use bevy_math::IVec3;
use ferrumc_macros::match_block;
use ferrumc_world::block_state_id::BlockStateId;
use ferrumc_world::chunk::Chunk;
use ferrumc_world::pos::ChunkBlockPos;

/// A fluid an entity can be submerged in, together with the drag it imposes.
///
/// Shared by the gravity, drag, and swim-to-surface systems so they agree on which fluids exist
/// and how strongly each one resists motion.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fluid {
    Water,
    Lava,
}

impl Fluid {
    /// Per-tick velocity multiplier applied while submerged.
    ///
    /// A smaller multiplier removes more velocity each tick. Lava is more viscous than water, so it
    /// damps motion harder. Mirrors vanilla `LivingEntity.travel()`: 0.8 in water, 0.5 in lava.
    pub const fn drag(self) -> f32 {
        match self {
            Fluid::Water => 0.8,
            Fluid::Lava => 0.5,
        }
    }
}

/// Returns the fluid occupying the given block position, if any.
///
/// Callers pass the hitbox centre, so sampling a single point keeps the gravity, drag, and
/// swim-to-surface systems in agreement on where "in fluid" begins, leaving no dead band at the
/// surface.
pub fn fluid_at(chunk: &Chunk, pos: IVec3) -> Option<Fluid> {
    let block = chunk.get_block(ChunkBlockPos::from(pos));
    if match_block!("water", block) {
        Some(Fluid::Water)
    } else if match_block!("lava", block) {
        Some(Fluid::Lava)
    } else {
        None
    }
}
