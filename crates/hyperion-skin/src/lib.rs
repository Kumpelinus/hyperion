#![feature(iter_intersperse)]

use flecs_ecs::{core::World, macros::Component, prelude::Module};

mod component;
mod system;

pub use component::Skin;

#[derive(Component)]
pub struct SkinModule;

impl Module for SkinModule {
    fn module(world: &World) {
        world.import::<component::SkinComponentModule>();
        world.import::<system::SkinSystemModule>();
    }
}


