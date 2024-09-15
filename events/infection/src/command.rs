#![expect(clippy::needless_pass_by_value)]

use std::borrow::Cow;

use flecs_ecs::core::{EntityViewGet, QueryBuilderImpl, SystemAPI, TermBuilderImpl, World};
use hyperion::{
    component::{
        command::{get_root_command, Command, Parser},
        InGameName, Uuid,
    },
    event,
    event::EventQueue,
    net::{Compose, NetworkStreamRef},
    system::player_join_world::list::{PlayerListActions, PlayerListEntry, PlayerListS2c},
    uuid, valence_protocol,
    valence_protocol::{
        game_mode::OptGameMode,
        ident,
        packets::{
            play,
            play::{player_abilities_s2c::PlayerAbilitiesFlags, PlayerAbilitiesS2c},
        },
        text::IntoText,
        GameMode, VarInt,
    },
    SystemId, SystemRegistry,
};
use tracing::{debug, trace_span};

use crate::{command::parse::ParsedCommand, component::team::Team};

mod parse;

pub fn add_to_tree(world: &World) {
    let root_command = get_root_command();

    // add to tree
    world
        .entity()
        .set(Command::literal("team"))
        .child_of_id(root_command);

    world
        .entity()
        .set(Command::literal("zombie"))
        .child_of_id(root_command);

    let speed = world
        .entity()
        .set(Command::literal("speed"))
        .child_of_id(root_command);

    world
        .entity()
        .set(Command::argument("amount", Parser::Float {
            min: Some(0.0),
            max: Some(1024.0),
        }))
        .child_of_id(speed);
}

struct CommandContext<'a> {
    stream: NetworkStreamRef,
    team: &'a mut Team,
    compose: &'a Compose,
    world: &'a World,
    system_id: SystemId,
    uuid: uuid::Uuid,
    name: &'a InGameName,
}

pub fn process(world: &World, registry: &mut SystemRegistry) {
    let system_id = registry.register();

    world
        .system_named::<(&Compose, &mut EventQueue<event::Command>)>(
            "handle_infection_events_player",
        )
        .term_at(0)
        .singleton()
        .term_at(1)
        .singleton()
        .multi_threaded()
        .each_iter(move |it, _, (compose, event_queue)| {
            let span = trace_span!("handle_infection_events_player");
            let _enter = span.enter();

            let world = it.world();
            for event in event_queue.drain() {
                let executed = event.raw.as_str();

                debug!("executed: {executed}");

                let Ok((_, command)) = parse::command(executed) else {
                    return;
                };

                world
                    .entity_from_id(event.by)
                    .get::<(&NetworkStreamRef, &mut Team, &Uuid, &InGameName)>(
                        |(stream, team, uuid, name)| {
                            let context = CommandContext {
                                stream: *stream,
                                team,
                                compose,
                                world: &world,
                                system_id,
                                uuid: uuid.0,
                                name,
                            };
                            process_command(command, context);
                        },
                    );
            }
        });
}

fn process_command(command: ParsedCommand, context: CommandContext) {
    match command {
        ParsedCommand::Speed(amount) => handle_speed_command(amount, context),
        ParsedCommand::Team => handle_team_command(context),
        ParsedCommand::Zombie => handle_zombie_command(context),
    }
}

fn handle_speed_command(amount: f32, context: CommandContext) {
    let msg = format!("Setting speed to {amount}");
    let pkt = play::GameMessageS2c {
        chat: msg.into_cow_text(),
        overlay: false,
    };

    context
        .compose
        .unicast(&pkt, context.stream, context.system_id, context.world)
        .unwrap();

    let pkt = fly_speed_packet(amount);
    context
        .compose
        .unicast(&pkt, context.stream, context.system_id, context.world)
        .unwrap();
}

fn handle_team_command(context: CommandContext) {
    let msg = format!("You are now on team {}", context.team);
    let text = play::GameMessageS2c {
        chat: msg.into_cow_text(),
        overlay: false,
    };
    context
        .compose
        .unicast(&text, context.stream, context.system_id, context.world)
        .unwrap();
}

fn handle_zombie_command(context: CommandContext) {
    static ZOMBIE_PROPERTY: std::sync::LazyLock<valence_protocol::profile::Property> =
        std::sync::LazyLock::new(|| {
            let skin = include_str!("zombie_skin.json");
            let json: serde_json::Value = serde_json::from_str(skin).unwrap();

            let value = json["textures"].as_str().unwrap().to_string();
            let signature = json["signature"].as_str().unwrap().to_string();

            valence_protocol::profile::Property {
                name: "textures".to_string(),
                value,
                signature: Some(signature),
            }
        });

    let msg = "Turning to zombie";

    // todo: maybe this should be an event?
    let text = play::GameMessageS2c {
        chat: msg.into_cow_text(),
        overlay: false,
    };
    context
        .compose
        .unicast(&text, context.stream, context.system_id, context.world)
        .unwrap();

    let uuids = &[context.uuid];
    // remove from list
    let pkt = play::PlayerRemoveS2c {
        uuids: Cow::Borrowed(uuids),
    };

    context
        .compose
        .unicast(&pkt, context.stream, context.system_id, context.world)
        .unwrap();

    let zombie = &*ZOMBIE_PROPERTY;
    let property = core::slice::from_ref(zombie);

    let singleton_entry = &[PlayerListEntry {
        player_uuid: context.uuid,
        username: Cow::Borrowed(context.name),
        properties: Cow::Borrowed(property),
        chat_data: None,
        listed: true,
        ping: 20,
        game_mode: GameMode::Survival,
        display_name: Some(context.name.to_string().into_cow_text()),
    }];

    let pkt = PlayerListS2c {
        actions: PlayerListActions::default().with_add_player(true),
        entries: Cow::Borrowed(singleton_entry),
    };

    context
        .compose
        .unicast(&pkt, context.stream, context.system_id, context.world)
        .unwrap();

    // first we need to respawn the player
    let pkt = play::PlayerRespawnS2c {
        dimension_type_name: ident!("minecraft:overworld").into(),
        dimension_name: ident!("minecraft:overworld").into(),
        hashed_seed: 0,
        game_mode: GameMode::Adventure,
        previous_game_mode: OptGameMode::default(),
        is_debug: false,
        is_flat: false,
        copy_metadata: false,
        last_death_location: None,
        portal_cooldown: VarInt::default(),
    };

    context
        .compose
        .unicast(&pkt, context.stream, context.system_id, context.world)
        .unwrap();
}

fn fly_speed_packet(amount: f32) -> PlayerAbilitiesS2c {
    PlayerAbilitiesS2c {
        flags: PlayerAbilitiesFlags::default()
            .with_allow_flying(true)
            .with_flying(true),
        flying_speed: amount,
        fov_modifier: 0.0,
    }
}