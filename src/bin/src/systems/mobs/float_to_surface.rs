use crate::systems::physics::fluid::fluid_at;
use bevy_ecs::prelude::{Query, Res, With};
use bevy_math::IVec3;
use ferrumc_core::transform::position::Position;
use ferrumc_core::transform::velocity::Velocity;
use ferrumc_entities::components::{Baby, EntityMetadata, PhysicalRegistry};
use ferrumc_entities::markers::CanFloat;
use ferrumc_state::GlobalStateResource;
use ferrumc_world::pos::ChunkPos;

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

        // Swim up in any fluid the body centre is submerged in (water or lava).
        // The swim impulse is multiplied by the fluid's drag factor to compensate for running after
        // the physics pipeline — drag would otherwise damp this force only on the next tick, making
        // it 25% stronger in water and 100% stronger in lava.
        if let Some(fluid) = fluid_at(&chunk, center_pos) {
            vel.vec.y += SWIM_UP_FORCE * fluid.drag();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::physics::fluid::Fluid;
    use bevy_ecs::prelude::*;
    use bevy_math::{DVec3, Vec3A};
    use ferrumc_data::generated::entities::EntityType as VanillaEntityType;
    use ferrumc_macros::block;
    use ferrumc_state::create_test_state;
    use ferrumc_world::block_state_id::BlockStateId;

    /// Fills the chunk at `chunk_pos` entirely with the given block. Tests pass an explicit block
    /// (water, lava, or air) so the result does not depend on non-deterministic generated terrain.
    fn fill_chunk(state: &GlobalStateResource, chunk_pos: ChunkPos, fill: BlockStateId) {
        let mut chunk =
            ferrumc_utils::world::load_or_generate_mut(&state.0, chunk_pos, "overworld")
                .expect("Failed to load or generate chunk");
        chunk.fill(fill);
    }

    fn spawn_floater(world: &mut World) -> Entity {
        world
            .spawn((
                Velocity { vec: Vec3A::ZERO },
                Position {
                    coords: DVec3::new(0.0, 65.0, 0.0),
                },
                EntityMetadata::from_vanilla(&VanillaEntityType::PIG),
                CanFloat,
            ))
            .id()
    }

    fn run(world: &mut World) {
        let mut schedule = Schedule::default();
        schedule.add_systems(float_to_surface);
        schedule.run(world);
    }

    #[test]
    fn floats_up_when_submerged_in_water() {
        let mut world = World::new();
        let (state, _temp_dir) = create_test_state();
        fill_chunk(&state, ChunkPos::new(0, 0), block!("water", { level: 0 }));
        world.insert_resource(state);
        world.insert_resource(PhysicalRegistry::new());

        let entity = spawn_floater(&mut world);
        run(&mut world);

        let vel = world.get::<Velocity>(entity).unwrap();
        assert!(
            (vel.vec.y - SWIM_UP_FORCE * Fluid::Water.drag()).abs() < 1e-6,
            "submerged floater should gain {SWIM_UP_FORCE} * water drag upward velocity, got {}",
            vel.vec.y
        );
    }

    #[test]
    fn floats_up_when_submerged_in_lava() {
        let mut world = World::new();
        let (state, _temp_dir) = create_test_state();
        fill_chunk(&state, ChunkPos::new(0, 0), block!("lava", { level: 0 }));
        world.insert_resource(state);
        world.insert_resource(PhysicalRegistry::new());

        let entity = spawn_floater(&mut world);
        run(&mut world);

        let vel = world.get::<Velocity>(entity).unwrap();
        assert!(
            (vel.vec.y - SWIM_UP_FORCE * Fluid::Lava.drag()).abs() < 1e-6,
            "a floater submerged in lava should swim up with lava-damped force, got {}",
            vel.vec.y
        );
    }

    #[test]
    fn does_not_float_out_of_fluid() {
        let mut world = World::new();
        let (state, _temp_dir) = create_test_state();
        fill_chunk(&state, ChunkPos::new(0, 0), block!("air"));
        world.insert_resource(state);
        world.insert_resource(PhysicalRegistry::new());

        let entity = spawn_floater(&mut world);
        run(&mut world);

        let vel = world.get::<Velocity>(entity).unwrap();
        assert_eq!(
            vel.vec.y, 0.0,
            "a floater out of fluid should not gain upward velocity"
        );
    }
}
