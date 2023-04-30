use std::f32::consts::PI;

use bevy::{
    core_pipeline::bloom::BloomSettings,
    input::mouse::MouseMotion,
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
    scene::SceneInstance,
    window::CursorGrabMode,
};
use bevy_asset_loader::prelude::*;
use bevy_rapier3d::prelude::*;

fn main() {
    App::new()
        .add_state::<GameState>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        //.add_plugin(RapierDebugRenderPlugin::default())
        .insert_resource(RapierConfiguration {
            timestep_mode: TimestepMode::Variable {
                max_dt: 1.0 / 20.0,
                time_scale: 1.0,
                substeps: 8,
            },
            ..default()
        })
        //.add_plugin(WorldInspectorPlugin::new())
        .insert_resource(ClearColor(Color::BLACK))
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading).continue_to_state(GameState::PrepareScene),
        )
        .add_collection_to_loading_state::<_, GameAssets>(GameState::AssetLoading)
        .add_systems(
            (grab_cursor, setup_graphics, spawn_player, player_ui)
                .in_schedule(OnEnter(GameState::PrepareScene)),
        )
        .add_system(add_scene_colliders.in_set(OnUpdate(GameState::PrepareScene)))
        .add_system(soundtrack.in_schedule(OnExit(GameState::AssetLoading)))
        .add_systems(
            (
                player_movement,
                player_gravity,
                player_jump,
                player_hold,
                reset_request,
                check_reach_objective,
                anvil_held,
                toggle_help,
                hit_events,
            )
                .chain()
                .in_set(OnUpdate(GameState::Play)),
        )
        .add_system(reset.in_schedule(OnExit(GameState::Play)))
        .init_resource::<AudioMixer>()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .insert_resource(Info {
            obj_info: vec![
                ObjectiveInfo {
                    powerup: Powerup::Weight,
                    description: "You feel more attracted to the anvil!".to_string(),
                },
                ObjectiveInfo {
                    powerup: Powerup::Strenght,
                    description: "Throw power increased!".to_string(),
                },
                ObjectiveInfo {
                    powerup: Powerup::Speed,
                    description: "Speed increased!".to_string(),
                },
            ],
        })
        .insert_resource(Progress {
            objectives: vec![],
            powerups: vec![],
            help: true,
        })
        .run();
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum GameState {
    #[default]
    AssetLoading,
    PrepareScene,
    Play,
}

#[derive(AssetCollection, Resource)]
struct GameAssets {
    #[asset(path = "ost.ogg")]
    ost: Handle<AudioSource>,
    #[asset(path = "hit.ogg")]
    hit: Handle<AudioSource>,
    #[asset(path = "testcity.gltf#Scene0")]
    testcity: Handle<Scene>,
    #[asset(path = "anvil.gltf#Scene0")]
    anvil: Handle<Scene>,
    #[asset(path = "monof55.ttf")]
    font: Handle<Font>,
}

#[derive(Resource, Default, Debug)]
struct AudioMixer {
    ost: Handle<AudioSink>,
}

fn soundtrack(game_assets: Res<GameAssets>, audio: Res<Audio>, mut mixer: ResMut<AudioMixer>) {
    mixer.ost = audio.play_with_settings(
        game_assets.ost.clone_weak(),
        PlaybackSettings {
            repeat: true,
            ..default()
        },
    );
}

fn hit_events(
    mut contact_force_events: EventReader<ContactForceEvent>,
    game_assets: Res<GameAssets>,
    audio: Res<Audio>,
) {
    for contact_force_event in contact_force_events.iter() {
        let vol = contact_force_event.total_force.length();
        let vol = (vol / 10000.0).clamp(0.0, 1.0);
        audio.play_with_settings(
            game_assets.hit.clone_weak(),
            PlaybackSettings {
                volume: vol,
                ..default()
            },
        );
    }
}

const HELPTEXT: &str = "\
Welcome, courier! This is the forge of Anvil Express.
You have to deliver an anvil to three customers.

Pick it up with [E].
Hold [E] to be attracted to the anvil.
Throw it with [Q] if you are not jumping.
If you lose it, just press [Delete].

The customers are identified by a blue light.
Good luck!

[H] to toggle this box.
";

#[derive(Component)]
struct HelpTag;

fn player_ui(mut commands: Commands, game_assets: Res<GameAssets>, progress: Res<Progress>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    position: UiRect::new(
                        Val::Percent(0.0),
                        Val::Percent(0.0),
                        Val::Percent(95.0),
                        Val::Percent(0.0),
                    ),
                    size: Size::new(Val::Percent(100.0), Val::Percent(5.0)),
                    ..default()
                },
                ..default()
            },
            Transient::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle {
                    text: Text::from_section(
                        "[H] of help",
                        TextStyle {
                            font: game_assets.font.clone(),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    ),
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    ..default()
                },
                Transient::default(),
            ));
        });
    let status = if progress.objectives.len() == 3 {
        "Congratulations, you have delivered all the anvils.".to_string()
    } else {
        format!("Anvils delivered {}/3", progress.objectives.len())
    };
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    position: UiRect::new(
                        Val::Percent(0.0),
                        Val::Percent(0.0),
                        Val::Percent(0.0),
                        Val::Percent(0.0),
                    ),
                    size: Size::new(Val::Percent(100.0), Val::Percent(10.0)),
                    ..default()
                },
                ..default()
            },
            Transient::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle {
                    text: Text::from_section(
                        status,
                        TextStyle {
                            font: game_assets.font.clone(),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    ),
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    ..default()
                },
                Transient::default(),
            ));
        });
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    position: UiRect::new(
                        Val::Percent(5.0),
                        Val::Percent(0.0),
                        Val::Percent(50.0),
                        Val::Percent(0.0),
                    ),
                    size: Size::new(Val::Percent(90.0), Val::Percent(45.0)),
                    ..default()
                },
                visibility: if progress.help {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
                ..default()
            },
            HelpTag,
            Transient::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle {
                    text: Text::from_section(
                        HELPTEXT,
                        TextStyle {
                            font: game_assets.font.clone(),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    ),
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    background_color: Color::rgba(0.0, 0.0, 0.0, 0.99).into(),
                    ..default()
                },
                Transient::default(),
            ));
        });
}

fn toggle_help(
    mut commands: Commands,
    mut progress: ResMut<Progress>,
    mut help_query: Query<(Entity, &HelpTag)>,
    keys: Res<Input<KeyCode>>,
) {
    if let Ok((ent, _)) = help_query.get_single_mut() {
        if keys.just_pressed(KeyCode::H) {
            if progress.help {
                progress.help = false;
                commands.entity(ent).insert(Visibility::Hidden);
            } else {
                progress.help = true;
                commands.entity(ent).insert(Visibility::Visible);
            }
        }
    }
}

fn grab_cursor(mut window_query: Query<&mut Window>) {
    if let Ok(mut window) = window_query.get_single_mut() {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
}

fn setup_graphics(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                color: Color::rgb(1.0, 0.97, 0.97),
                ..default()
            },
            cascade_shadow_config: CascadeShadowConfigBuilder {
                num_cascades: 4,
                maximum_distance: 2000.0,
                ..default()
            }
            .into(),
            transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.5, 0.0)),
            ..default()
        },
        Transient::default(),
    ));
    commands.spawn((
        SceneBundle {
            scene: game_assets.testcity.clone(),
            transform: Transform::default().with_scale(Vec3::splat(1.0)),
            ..default()
        },
        Transient::default(),
    ));
}

fn add_scene_colliders(
    mut commands: Commands,
    scene_query: Query<(Entity, &SceneInstance)>,
    children: Query<&Children>,
    has_mesh: Query<(&Transform, &GlobalTransform, &Handle<Mesh>)>,
    has_name: Query<&Name>,
    meshes: ResMut<Assets<Mesh>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if scene_query.is_empty() {
        return;
    }
    // very ugly, wait for anvil + testcity
    if children
        .iter_descendants(scene_query.iter().next().unwrap().0)
        .count()
        < 2
    {
        return;
    }

    for (scene, _) in scene_query.iter() {
        for descendant in children.iter_descendants(scene) {
            if let Ok((transform, gt, mesh)) = has_mesh.get(descendant) {
                let rapier_collider = Collider::from_bevy_mesh(
                    meshes.get(mesh).unwrap(),
                    &ComputedColliderShape::TriMesh,
                )
                .unwrap();
                // ugly, but will do
                if let Ok(name) = has_name.get(scene) {
                    if name.as_ref() == "Anvil" {
                        commands.entity(scene).insert(Transient::default());
                        commands.entity(descendant).insert(Transient::default());
                    }
                } else {
                    if let Ok(name) = has_name.get(descendant) {
                        if let Some(num) = name.as_ref().split("Objective").skip(1).next() {
                            commands.spawn((
                                TransformBundle {
                                    local: Transform::from_translation(gt.translation()),
                                    ..default()
                                },
                                Objective {
                                    num: num.parse().unwrap(),
                                },
                            ));
                        }
                    }
                    commands.entity(descendant).insert(rapier_collider);
                }
            }
        }
    }
    next_state.set(GameState::Play);
}

#[derive(Component, Default, Clone, Debug)]
struct Player {
    speed: f32,
    velocity: Vec3,
    jump_strenght: f32,
    pickup_distance: f32,
    cooldown: Timer,
    launched: bool,
}

#[derive(Component, Default, Clone, Debug)]
struct PlayerCamera {
    sensitivity: Vec3,
}

#[derive(Component, Default, Clone, Debug)]
struct Anvil {}

#[derive(Component, Default, Clone, Debug)]
struct Held {}

#[derive(Component, Default, Clone, Debug)]
struct Transient {}

#[derive(Component, Default, Clone, Debug)]
struct Objective {
    num: u32,
}

#[derive(Resource, Default, Clone, Debug)]
struct Progress {
    objectives: Vec<u32>,
    powerups: Vec<Powerup>,
    help: bool,
}

#[derive(Resource, Default, Clone, Debug)]
struct Info {
    obj_info: Vec<ObjectiveInfo>,
}

#[derive(Default, Clone, Debug)]
struct ObjectiveInfo {
    description: String,
    powerup: Powerup,
}

#[derive(Component, Default, Clone, Debug, PartialEq)]
enum Powerup {
    #[default]
    Strenght,
    Weight,
    Speed,
}

fn spawn_player(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn((
        Anvil::default(),
        Name::new("Anvil"),
        Velocity::default(),
        RigidBody::Dynamic,
        SceneBundle {
            scene: game_assets.anvil.clone(),
            transform: Transform::from_xyz(0.0, 12.5, 0.0),
            ..default()
        },
        ColliderMassProperties::Density(10.0),
        CollisionGroups::new(Group::GROUP_2, Group::ALL),
        Collider::cuboid(0.5, 0.4, 0.3),
        Friction {
            coefficient: 0.01,
            combine_rule: CoefficientCombineRule::Min,
        },
        Restitution {
            coefficient: 0.02,
            combine_rule: CoefficientCombineRule::Min,
        },
        Damping {
            linear_damping: 0.2,
            angular_damping: 10.0,
        },
        Sleeping::disabled(),
        Transient::default(),
        ActiveEvents::CONTACT_FORCE_EVENTS,
        ContactForceEventThreshold(3000.0),
    ));
    commands
        .spawn((
            Name::new("Player"),
            Player {
                speed: 1.0,
                velocity: Vec3::ZERO,
                jump_strenght: 0.07,
                pickup_distance: 3.0,
                cooldown: Timer::from_seconds(0.3, TimerMode::Once),
                launched: false,
            },
            TransformBundle {
                local: Transform::from_xyz(0.0, 12.0, 2.0),
                ..default()
            },
            Velocity::default(),
            RigidBody::KinematicPositionBased,
            KinematicCharacterController {
                max_slope_climb_angle: 45.0_f32.to_radians(),
                min_slope_slide_angle: 30.0_f32.to_radians(),
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Absolute(0.5),
                    min_width: CharacterLength::Absolute(0.2),
                    include_dynamic_bodies: true,
                }),
                ..default()
            },
            Collider::capsule(Vec3::new(0.0, 0.25, 0.0), Vec3::new(0.0, 1.5, 0.0), 0.25),
            CollisionGroups::new(Group::GROUP_1, Group::ALL),
            Transient::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Camera3dBundle {
                    transform: Transform::from_xyz(0.0, 1.5, 0.0),
                    projection: Projection::Perspective(PerspectiveProjection {
                        fov: PI / 2.0,
                        ..default()
                    }),
                    camera: Camera {
                        hdr: true,
                        ..default()
                    },
                    ..default()
                },
                BloomSettings {
                    intensity: 0.1, // the default is 0.3
                    ..default()
                },
                PlayerCamera {
                    sensitivity: Vec3::new(0.004, 0.004, 1.0),
                },
                Transient::default(),
            ));
        });
}

fn player_movement(
    mut player_query: Query<
        (
            &mut Player,
            &mut Transform,
            &mut KinematicCharacterController,
        ),
        Without<PlayerCamera>,
    >,
    anvil_held_query: Query<(Entity, &Anvil, &Held)>,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut cam_query: Query<(&mut PlayerCamera, &mut Transform), Without<Player>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    progress: Res<Progress>,
) {
    if let Ok((mut player, mut tr, mut contr)) = player_query.get_single_mut() {
        if let Ok((cam, mut cam_tr)) = cam_query.get_single_mut() {
            for mov in mouse_motion_events.iter() {
                let pitch = cam_tr.rotation.to_euler(EulerRot::XYZ).0;
                let amt = -mov.delta.y * cam.sensitivity.y;
                if pitch + amt > -PI / 2.0 && pitch + amt < PI / 2.0 {
                    cam_tr.rotate_local_x(amt);
                }
                tr.rotate_y(-mov.delta.x * cam.sensitivity.y);
            }

            let mut acceleration = Vec3::new(0.0, 0.0, 0.0);
            if keys.pressed(KeyCode::W) {
                acceleration += tr.forward();
            }
            if keys.pressed(KeyCode::S) {
                acceleration -= tr.forward();
            }
            if keys.pressed(KeyCode::D) {
                acceleration += tr.right();
            }
            if keys.pressed(KeyCode::A) {
                acceleration -= tr.right();
            }
            if acceleration.length_squared() > 1.0 {
                acceleration = acceleration.normalize()
            }
            if progress.powerups.contains(&Powerup::Speed) {
                acceleration *= 1.2;
            }
            acceleration *= player.speed;
            if !anvil_held_query.is_empty() {
                acceleration *= 0.5;
            }
            acceleration += Vec3::NEG_Y * 0.3;
            player.velocity += acceleration * time.delta_seconds();

            contr.translation = Some(player.velocity * time.delta_seconds() * 100.0);
        }
    }
}

fn player_gravity(
    mut player_query: Query<(&mut Player, &KinematicCharacterControllerOutput)>,
    anvil_held_query: Query<(Entity, &Anvil, &Held)>,
) {
    if let Ok((mut player, out)) = player_query.get_single_mut() {
        if out.grounded {
            player.velocity.y = 0.0;
            player.velocity.x *= 0.9;
            player.velocity.z *= 0.9;
        } else if anvil_held_query.is_empty() {
            player.velocity.x *= 0.93;
            player.velocity.z *= 0.93;
        } else {
            player.velocity.x *= 0.99;
            player.velocity.z *= 0.99;
        }
    }
}

fn player_jump(
    mut player_query: Query<(&mut Player, &KinematicCharacterControllerOutput)>,
    anvil_held_query: Query<(Entity, &Anvil, &Held)>,
    keys: Res<Input<KeyCode>>,
) {
    if keys.pressed(KeyCode::Space) {
        if let Ok((mut player, out)) = player_query.get_single_mut() {
            let on_held_anvil = out
                .collisions
                .iter()
                .any(|coll| anvil_held_query.get(coll.entity).is_ok());
            if out.grounded && !on_held_anvil {
                player.velocity.y = player.jump_strenght;
            }
        }
    }
}

fn player_hold(
    mut commands: Commands,
    mut player_query: Query<
        (&mut Player, &Transform, &KinematicCharacterControllerOutput),
        (Without<Anvil>, Without<PlayerCamera>),
    >,
    mut cam_query: Query<(&mut PlayerCamera, &GlobalTransform), (Without<Anvil>, Without<Player>)>,
    mut anvil_query: Query<
        (Entity, &mut Anvil, &Transform, &mut Velocity),
        (Without<Player>, Without<PlayerCamera>),
    >,
    held_query: Query<&Held>,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    progress: Res<Progress>,
) {
    match (player_query.get_single_mut(), anvil_query.get_single_mut()) {
        (Ok((mut player, tr_player, out)), Ok((anvil_ent, _anvil, tr_anvil, mut vel))) => {
            player.cooldown.tick(time.delta());
            if held_query.get(anvil_ent).is_err() {
                if keys.pressed(KeyCode::E) && player.cooldown.finished() {
                    let mut delta = tr_anvil.translation - (tr_player.translation + Vec3::Y);
                    if delta.length_squared() < player.pickup_distance * player.pickup_distance {
                        // pickup
                        commands.entity(anvil_ent).insert((
                            Held::default(),
                            RigidBody::Fixed,
                            CollisionGroups::new(Group::NONE, Group::NONE),
                        ));
                    } else {
                        // attract
                        //if player.velocity.length_squared() > 1.0 {
                        //    delta = delta.normalize() / player.velocity.length_squared();
                        //}
                        let mut amt = 0.003;
                        if progress.powerups.contains(&Powerup::Weight) {
                            amt = 0.004;
                        }
                        amt *= time.delta_seconds() * 100.0;
                        vel.linvel -= delta * amt * 0.2;
                        player.velocity += delta * amt;
                    }
                }
            } else {
                if keys.just_pressed(KeyCode::E) {
                    // put down
                    player.cooldown.reset();
                    commands
                        .entity(anvil_ent)
                        .insert((
                            RigidBody::Dynamic,
                            Velocity {
                                linvel: player.velocity * 100.0,
                                ..default()
                            },
                            CollisionGroups::new(Group::GROUP_2, Group::ALL),
                        ))
                        .remove::<Held>();
                } else if keys.pressed(KeyCode::Q) && !player.launched {
                    // throw
                    player.cooldown.reset();
                    let on_held_anvil = out
                        .collisions
                        .iter()
                        .any(|coll| held_query.get(coll.entity).is_ok());
                    if out.grounded && !on_held_anvil {
                        if let Ok((_, tr_cam)) = cam_query.get_single_mut() {
                            let mut str = 10.0;
                            if progress.powerups.contains(&Powerup::Strenght) {
                                str = 12.0;
                            }
                            commands
                                .entity(anvil_ent)
                                .insert((
                                    RigidBody::Dynamic,
                                    Velocity {
                                        linvel: player.velocity * 80.0 + tr_cam.forward() * str,
                                        ..default()
                                    },
                                    CollisionGroups::new(Group::GROUP_2, Group::ALL),
                                ))
                                .remove::<Held>();
                        }
                    }
                }
            }
        }
        (_, _) => {}
    }
}

fn anvil_held(
    mut player_query: Query<(&Player, &Transform), Without<Anvil>>,
    mut anvil_query: Query<(&mut Anvil, &mut Transform, &Held), Without<Player>>,
) {
    match (player_query.get_single_mut(), anvil_query.get_single_mut()) {
        (Ok((_player, tr_player)), Ok((_anvil, mut tr_anvil, _held))) => {
            let off = tr_player.forward() * 1.2 + tr_player.up() * 0.85;
            tr_anvil.translation = tr_player.translation + off;
            let mut angle = tr_player.rotation.to_euler(EulerRot::XYZ).1;
            // i'm stupid, can't figure out why this is needed
            if tr_player.forward().dot(Vec3::Z) > 0.0 {
                angle = PI - angle;
            }
            tr_anvil.rotation = Quat::from_rotation_y(angle);
        }
        (_, _) => {}
    }
}

fn check_reach_objective(
    obj_query: Query<(&Objective, &Transform), Without<Anvil>>,
    anvil_query: Query<(&Anvil, &Transform), Without<Objective>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut progress: ResMut<Progress>,
    info: ResMut<Info>,
) {
    if let Ok((_, tr_anvil)) = anvil_query.get_single() {
        for (obj, tr_current_obj) in obj_query.iter() {
            if progress.objectives.contains(&obj.num) {
                continue;
            }
            let delta = tr_anvil.translation - (tr_current_obj.translation + Vec3::Y);
            if delta.length_squared() < 25.0 {
                next_state.set(GameState::PrepareScene);
                progress.objectives.push(obj.num);
                // lol useful
                if obj.num < 3 {
                    progress
                        .powerups
                        .push(info.obj_info[obj.num as usize].powerup.clone());
                }
            }
        }
    }
}

fn reset_request(keys: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::Delete) {
        next_state.set(GameState::PrepareScene);
    }
}

fn cheat_powerup(keys: Res<Input<KeyCode>>, mut progress: ResMut<Progress>, info: ResMut<Info>) {
    if keys.just_pressed(KeyCode::Key1) {
        progress
            .powerups
            .push(info.obj_info[0 as usize].powerup.clone());
    }
    if keys.just_pressed(KeyCode::Key2) {
        progress
            .powerups
            .push(info.obj_info[1 as usize].powerup.clone());
    }
    if keys.just_pressed(KeyCode::Key2) {
        progress
            .powerups
            .push(info.obj_info[2 as usize].powerup.clone());
    }
}

fn reset(mut commands: Commands, query: Query<Entity, &Transient>) {
    for ent in query.iter() {
        commands.entity(ent).despawn()
    }
}
