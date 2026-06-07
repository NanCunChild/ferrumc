use crate::systems::physics::fluid::{fluid_at, Fluid};
use bevy_ecs::prelude::{Has, Or, Query, Res, With};
use bevy_math::{IVec3, Vec3A};
use ferrumc_core::transform::grounded::OnGround;
use ferrumc_core::transform::position::Position;
use ferrumc_core::transform::velocity::Velocity;
use ferrumc_entities::components::{Baby, EntityMetadata, PhysicalRegistry};
use ferrumc_entities::markers::{HasLavaDrag, HasWaterDrag};
use ferrumc_state::GlobalStateResource;
use ferrumc_world::pos::ChunkPos;

const GROUND_FRICTION: f32 = 0.6;
const AIR_FRICTION: f32 = 0.91;
const AIR_DRAG: f32 = 0.98;
const STOP_THRESHOLD: f32 = 0.005;

type DragQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Velocity,
        &'static Position,
        &'static EntityMetadata,
        &'static OnGround,
        Option<&'static Baby>,
        Has<HasWaterDrag>,
        Has<HasLavaDrag>,
    ),
    Or<(With<HasWaterDrag>, With<HasLavaDrag>)>,
>;

pub fn handle(
    mut query: DragQuery,
    state: Res<GlobalStateResource>,
    registry: Res<PhysicalRegistry>,
) {
    for (mut vel, pos, metadata, on_ground, baby, has_water_drag, has_lava_drag) in query.iter_mut()
    {
        let is_baby = baby.is_some();
        let Some(physical) = registry.get(metadata.protocol_id(), is_baby) else {
            continue;
        };
        let chunk_pos = ChunkPos::from(pos.coords);
        let chunk = ferrumc_utils::world::load_or_generate_mut(&state.0, chunk_pos, "overworld")
            .expect("Failed to load or generate chunk");

        let feet_pos = pos.coords.as_ivec3();
        let center_y = pos.coords.y + (physical.bounding_box.height() / 2.0);
        let center_pos = IVec3::new(feet_pos.x, center_y as i32, feet_pos.z);

        let drag = match fluid_at(&chunk, center_pos) {
            Some(Fluid::Water) if has_water_drag => Some(Fluid::Water.drag()),
            Some(Fluid::Lava) if has_lava_drag => Some(Fluid::Lava.drag()),
            _ => None,
        };

        if let Some(drag) = drag {
            **vel *= Vec3A::splat(drag);
            // For submerged entities, fluid drag provides enough natural damping — no
            // stop threshold is applied here. Applying it would zero the -0.004
            // blocks/tick that reduced gravity produces through the drag multiplier,
            // preventing non-swimming entities from ever sinking.
        } else {
            let h_friction = if on_ground.0 {
                GROUND_FRICTION
            } else {
                AIR_FRICTION
            };
            vel.vec.x *= h_friction;
            vel.vec.y *= AIR_DRAG;
            vel.vec.z *= h_friction;

            if vel.vec.x.abs() < STOP_THRESHOLD {
                vel.vec.x = 0.0;
            }
            if vel.vec.y.abs() < STOP_THRESHOLD {
                vel.vec.y = 0.0;
            }
            if vel.vec.z.abs() < STOP_THRESHOLD {
                vel.vec.z = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;
    use bevy_math::DVec3;
    use ferrumc_data::generated::entities::EntityType as VanillaEntityType;
    use ferrumc_macros::block;
    use ferrumc_state::create_test_state;
    use ferrumc_world::block_state_id::BlockStateId;

    enum ChunkFill {
        Water,
        Lava,
    }

    fn fill_chunk(state: &GlobalStateResource, chunk_pos: ChunkPos, fill: ChunkFill) {
        let mut chunk =
            ferrumc_utils::world::load_or_generate_mut(&state.0, chunk_pos, "overworld")
                .expect("Failed to load or generate chunk");
        match fill {
            ChunkFill::Water => chunk.fill(block!("water", { level: 0 })),
            ChunkFill::Lava => chunk.fill(block!("lava", { level: 0 })),
        }
    }

    fn run_drag(fill: ChunkFill) -> f32 {
        let mut world = World::new();
        let (state, _temp_dir) = create_test_state();
        fill_chunk(&state, ChunkPos::new(0, 0), fill);
        world.insert_resource(state);
        world.insert_resource(PhysicalRegistry::new());

        let entity = world
            .spawn((
                Velocity {
                    vec: Vec3A::splat(1.0),
                },
                Position {
                    coords: DVec3::new(0.0, 65.0, 0.0),
                },
                EntityMetadata::from_vanilla(&VanillaEntityType::PIG),
                OnGround(false),
                HasWaterDrag,
                HasLavaDrag,
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(handle);
        schedule.run(&mut world);

        world.get::<Velocity>(entity).unwrap().vec.x
    }

    #[test]
    fn water_drag_is_applied() {
        assert!(
            (run_drag(ChunkFill::Water) - Fluid::Water.drag()).abs() < 1e-6,
            "submerged entity should be slowed by the water drag multiplier"
        );
    }

    #[test]
    fn lava_drag_is_stronger_than_water() {
        let lava = run_drag(ChunkFill::Lava);
        assert!(
            (lava - Fluid::Lava.drag()).abs() < 1e-6,
            "submerged entity should be slowed by the lava drag multiplier"
        );
        // A smaller remaining velocity means more was removed: lava is more viscous than water.
        assert!(
            lava < run_drag(ChunkFill::Water),
            "lava should damp motion harder than water"
        );
    }
}
