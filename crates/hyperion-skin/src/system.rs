use flecs_ecs::{
    core::{flecs, EntityViewGet, SystemAPI, World, WorldGet, WorldProvider}, macros::observer, prelude::{Component, Module}
};
use hyperion::{
    net::{Compose, DataBundle, NetworkStreamRef, agnostic},
    system_registry::SystemId,
};

use crate::component::Skin;

#[derive(Component)]
pub struct SkinSystemModule;

impl Module for SkinSystemModule {
    fn module(world: &World) {

        observer!(world, flecs::OnSet, &Skin).each_entity(|entity, skin| {
            let world = entity.world();

            //let cmd_pkt = get_command_packet(&world, root_command, Some(*entity));

            entity.get::<&NetworkStreamRef>(|stream| {
                world.get::<&Compose>(|compose| {
                    compose
                        .unicast(&cmd_pkt, *stream, SystemId(999), &world)
                        .unwrap();
                });
            });
        });
    }
}