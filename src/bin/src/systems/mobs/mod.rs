mod combat;
mod death;
mod float_to_surface;
mod pig;

pub fn register_mob_systems(schedule: &mut bevy_ecs::schedule::Schedule) {
    // pig::tick_pig is intentionally not registered yet.
    schedule.add_systems(combat::tick_combat);
    // `detect_deaths` tags newly dead entities `Dying`; `tick_dying` counts that timer down and
    // removes them. Their command effects (insert/despawn) are deferred to the schedule's sync
    // point, so the two are order-independent and need no explicit ordering.
    schedule.add_systems(death::detect_deaths);
    schedule.add_systems(death::tick_dying);
    // Runs after the physics pipeline (register_physics is registered before register_mob_systems),
    // so passive physics sinks a submerged mob first and this behaviour then adds the upward swim
    // intent. Buoyancy is intentionally a behaviour, not a physics force.
    schedule.add_systems(float_to_surface::float_to_surface);
}
