use bevy_ecs::prelude::{Has, Query, Res, With};
use bevy_math::IVec3;
use ferrumc_core::transform::grounded::OnGround;
use ferrumc_core::transform::position::Position;
use ferrumc_core::transform::velocity::Velocity;
use ferrumc_entities::components::{Baby, EntityMetadata, PhysicalRegistry};
use ferrumc_entities::markers::{HasGravity, HasWaterDrag};
use ferrumc_macros::match_block;
use ferrumc_physics::{GRAVITY_ACCELERATION, SUBMERGED_GRAVITY_FACTOR};
use ferrumc_state::GlobalStateResource;
use ferrumc_world::block_state_id::BlockStateId;
use ferrumc_world::pos::{ChunkBlockPos, ChunkPos};

type EntityQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Velocity,
        &'static OnGround,
        &'static Position,
        Has<HasWaterDrag>,
        Option<&'static EntityMetadata>,
        Option<&'static Baby>,
    ),
    With<HasGravity>,
>;

// Just apply gravity to a mob's velocity. Application of velocity is handled elsewhere.
pub(crate) fn handle(
    mut entities: EntityQuery,
    state: Res<GlobalStateResource>,
    registry: Option<Res<PhysicalRegistry>>,
) {
    for (mut vel, grounded, pos, is_water, metadata, baby) in entities.iter_mut() {
        if grounded.0 {
            continue;
        }

        if is_water {
            let chunk_pos = ChunkPos::from(pos.coords);
            let chunk =
                ferrumc_utils::world::load_or_generate_mut(&state.0, chunk_pos, "overworld")
                    .expect("Failed to load or generate chunk");

            let feet_pos = pos.coords.as_ivec3();

            // Check submersion at the body centre, the same point the water-drag system uses,
            // so gravity stops exactly where water drag takes over. Checking the feet instead
            // would leave a band — feet submerged but centre above the surface — in which neither
            // gravity nor water drag acts, letting a mob move freely. Entities without metadata
            // (e.g. in unit tests) fall back to the feet position.
            let submersion_pos = metadata
                .zip(registry.as_ref())
                .and_then(|(m, reg)| reg.get(m.protocol_id(), baby.is_some()))
                .map(|physical| {
                    let center_y = pos.coords.y + (physical.bounding_box.height() / 2.0);
                    IVec3::new(feet_pos.x, center_y as i32, feet_pos.z)
                })
                .unwrap_or(feet_pos);

            let is_submerged = match_block!(
                "water",
                chunk.get_block(ChunkBlockPos::from(submersion_pos))
            );

            // A submerged entity is not weightless: buoyancy is a behaviour (see the
            // swim-to-surface system), not a passive force, so it still sinks slowly under reduced
            // gravity until something swims it back up. Outside the fluid it falls normally.
            if is_submerged {
                vel.vec += GRAVITY_ACCELERATION * SUBMERGED_GRAVITY_FACTOR;
            } else {
                vel.vec += GRAVITY_ACCELERATION;
            }
        } else {
            // Apply gravity
            vel.vec += GRAVITY_ACCELERATION;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;
    use bevy_math::DVec3;
    use bevy_math::Vec3A;
    use ferrumc_core::transform::grounded::OnGround;
    use ferrumc_core::transform::velocity::Velocity;
    use ferrumc_entities::markers::HasGravity;
    use ferrumc_macros::block;
    use ferrumc_state::create_test_state;

    /// Creates a chunk with water blocks at the specified positions
    /// This helper function is used to set up test scenarios where entities are in water
    fn create_chunk_with_water(state: &GlobalStateResource, chunk_pos: ChunkPos) {
        // Load or generate the chunk
        let mut chunk =
            ferrumc_utils::world::load_or_generate_mut(&state.0, chunk_pos, "overworld")
                .expect("Failed to load or generate chunk");

        chunk.fill(block!("water", { level: 0 }));
    }

    #[test]
    fn test_gravity_application() {
        let mut world = World::new();
        let (state, _temp_dir) = create_test_state();
        world.insert_resource(state);

        let entity = world
            .spawn((
                Velocity { vec: Vec3A::ZERO },
                OnGround(false),
                Position {
                    coords: DVec3::new(0.0, 100.0, 0.0),
                },
                HasGravity,
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(handle);

        // Run the gravity system
        schedule.run(&mut world);

        let vel = world.get::<Velocity>(entity).unwrap();
        assert!(
            vel.vec.y < 0.0,
            "Velocity Y should be negative after gravity application"
        );
    }

    #[test]
    fn test_no_gravity_when_grounded() {
        let mut world = World::new();
        let (state, _temp_dir) = create_test_state();
        world.insert_resource(state);

        let entity = world
            .spawn((
                Velocity { vec: Vec3A::ZERO },
                OnGround(true),
                Position {
                    coords: DVec3::new(0.0, 100.0, 0.0),
                },
                HasGravity,
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(handle);

        // Run the gravity system
        schedule.run(&mut world);

        let vel = world.get::<Velocity>(entity).unwrap();
        assert_eq!(
            vel.vec.y, 0.0,
            "Velocity Y should remain zero when grounded"
        );
    }

    #[test]
    fn test_water_entity_gravity_not_in_water() {
        let mut world = World::new();
        let (state, _temp_dir) = create_test_state();
        world.insert_resource(state);

        let entity = world
            .spawn((
                Velocity { vec: Vec3A::ZERO },
                OnGround(false),
                Position {
                    coords: DVec3::new(0.0, 100.0, 0.0),
                },
                HasGravity,
                HasWaterDrag,
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(handle);

        // Run the gravity system
        schedule.run(&mut world);

        let vel = world.get::<Velocity>(entity).unwrap();
        assert!(
            vel.vec.y < 0.0,
            "Water entity should have gravity applied when not in water"
        );
    }

    #[test]
    fn test_water_entity_no_gravity_when_grounded() {
        let mut world = World::new();
        let (state, _temp_dir) = create_test_state();
        world.insert_resource(state);

        let entity = world
            .spawn((
                Velocity { vec: Vec3A::ZERO },
                OnGround(true),
                Position {
                    coords: DVec3::new(0.0, 100.0, 0.0),
                },
                HasGravity,
                HasWaterDrag,
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(handle);

        // Run the gravity system
        schedule.run(&mut world);

        let vel = world.get::<Velocity>(entity).unwrap();
        assert_eq!(
            vel.vec.y, 0.0,
            "Water entity should not have gravity when grounded"
        );
    }

    #[test]
    fn test_water_entity_in_water_reduced_gravity() {
        let mut world = World::new();
        let (state, _temp_dir) = create_test_state();

        // Create a chunk with water blocks
        let chunk_pos = ChunkPos::new(0, 0);
        create_chunk_with_water(&state, chunk_pos);

        world.insert_resource(state);

        // Spawn entity at Y=65 (in water)
        let entity = world
            .spawn((
                Velocity { vec: Vec3A::ZERO },
                OnGround(false),
                Position {
                    coords: DVec3::new(0.0, 65.0, 0.0),
                },
                HasGravity,
                HasWaterDrag,
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(handle);

        // Run the gravity system
        schedule.run(&mut world);

        // Submerged entities are not weightless — they sink slowly under reduced gravity rather
        // than freezing in place, so a swim behaviour can later carry them back up.
        let vel = world.get::<Velocity>(entity).unwrap();
        let expected = GRAVITY_ACCELERATION.y * SUBMERGED_GRAVITY_FACTOR;
        assert!(
            (vel.vec.y - expected).abs() < 1e-6,
            "Water entity should sink under reduced gravity ({expected}), got {}",
            vel.vec.y
        );
    }
}
