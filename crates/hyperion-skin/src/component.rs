use flecs_ecs::{
    core::World,
    macros::Component,
    prelude::Module,
};


#[derive(Component)]
pub struct Skin;


#[derive(Component)]
pub struct SkinComponentModule;

impl Module for SkinComponentModule {
    fn module(world: &World) {
        world.component::<Skin>();
    }
}