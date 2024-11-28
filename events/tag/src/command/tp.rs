use clap::Parser;
use flecs_ecs::core::{Entity, EntityViewGet, World, WorldGet};
use hyperion::{
    glam::{DVec3, Vec3}, net::{agnostic, Compose, DataBundle, NetworkStreamRef}, simulation::Position, system_registry::SystemId, valence_protocol::{packets::play::{
        player_position_look_s2c::PlayerPositionLookFlags, PlayerPositionLookS2c
    }, VarInt}
};
use hyperion_clap::{CommandPermission, MinecraftCommand};


#[derive(Parser, CommandPermission, Debug)]
#[command(name = "tp")]
#[command_permission(group = "Moderator")]
pub struct TpCommand;

impl MinecraftCommand for TpCommand {
    fn execute(self, world: &World, caller: Entity) {
        world.get::<&Compose>(|compose| {
            caller
                .entity_view(world)
                .get::<(&mut Position, &NetworkStreamRef)>(|(pos, stream)| {
                    let new_position = Position::from(Vec3::new(
                        0.0,
                        100.0,
                        0.0,
                    ));

                    let packet = position_packet(new_position.as_dvec3());

                    caller.entity_view(world).set(new_position);

                    let chat_packet = agnostic::chat(format!("§aTeleported to {} {} {}", pos.x, pos.y, pos.z));

                    let mut bundle = DataBundle::new(compose);

                    bundle.add_packet(&packet, world).unwrap();
                    bundle.add_packet(&chat_packet, world).unwrap();

                    bundle.send(world, *stream, SystemId(8)).unwrap();
                });
        });
    }
}

fn position_packet(pos: DVec3) -> PlayerPositionLookS2c {
    PlayerPositionLookS2c {
        position: pos,
        yaw: 0.0,
        pitch: 0.0,
        flags: PlayerPositionLookFlags::default(),
        teleport_id: VarInt(fastrand::i32(..)),
    }
}

