use bevy_ecs::prelude::{Query, Res, With};
use bevy_math::IVec3;
use ferrumc_core::transform::position::Position;
use ferrumc_core::transform::velocity::Velocity;
use ferrumc_entities::components::{Baby, EntityMetadata, PhysicalRegistry};
use ferrumc_entities::markers::CanFloat;
use ferrumc_macros::match_block;
use ferrumc_state::GlobalStateResource;
use ferrumc_world::block_state_id::BlockStateId;
use ferrumc_world::pos::{ChunkBlockPos, ChunkPos};

/// Upward velocity (blocks/tick) applied to a floating mob whose body centre is submerged.
///
/// Mirrors the vanilla in-water jump impulse used by `FloatGoal`: the mob rises until its centre
/// breaks the surface, at which point gravity resumes and it settles roughly half-submerged.
const SWIM_UP_FORCE: f32 = 0.04;

/// Generic "swim to the surface" behaviour.
///
/// This is a reusable behaviour rather than a per-entity-type system: any mob that should keep its
/// head above water carries the [`CanFloat`] marker and is handled here, while mobs that should
/// sink (for example the iron golem) simply omit the marker.
///
/// Registered after the physics pipeline so passive physics runs first — water suppresses gravity
/// and applies drag, sinking the mob — and only then does this behaviour add the upward intent for
/// the next tick. Keeping buoyancy out of the physics layer this way means no passive force ever
/// pushes an entity up; only a deliberate swim behaviour does.
///
/// Submersion is sampled at the body centre, matching the gravity and drag systems so all three
/// agree on where "in water" begins and no dead band remains around the surface.
pub fn float_to_surface(
    mut query: Query<(&mut Velocity, &Position, &EntityMetadata, Option<&Baby>), With<CanFloat>>,
    state: Res<GlobalStateResource>,
    registry: Res<PhysicalRegistry>,
) {
    for (mut vel, pos, metadata, baby) in query.iter_mut() {
        let Some(physical) = registry.get(metadata.protocol_id(), baby.is_some()) else {
            continue;
        };

        let chunk_pos = ChunkPos::from(pos.coords);
        let chunk = ferrumc_utils::world::load_or_generate_mut(&state.0, chunk_pos, "overworld")
            .expect("Failed to load or generate chunk");

        let feet_pos = pos.coords.as_ivec3();
        let center_y = pos.coords.y + (physical.bounding_box.height() / 2.0);
        let center_pos = IVec3::new(feet_pos.x, center_y as i32, feet_pos.z);

        let is_center_in_water =
            match_block!("water", chunk.get_block(ChunkBlockPos::from(center_pos)));

        if is_center_in_water {
            vel.vec.y += SWIM_UP_FORCE;
        }
    }
}
