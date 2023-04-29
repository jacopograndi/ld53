use std::f32::consts::PI;

use bevy::{
    input::mouse::MouseMotion,
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
    scene::SceneInstance,
    window::{Cursor, CursorGrabMode},
};
use bevy_asset_loader::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier3d::{prelude::*, render::RapierDebugRenderPlugin};

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
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(WorldInspectorPlugin::new())
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
        .add_system(soundtrack.in_schedule(OnEnter(GameState::Play)))
        .add_systems(
            (player_movement, player_gravity, player_jump, player_pickup)
                .chain()
                .in_set(OnUpdate(GameState::Play)),
        )
        .add_system(anvil_held.in_base_set(PhysicsSet::Writeback))
        .init_resource::<AudioMixer>()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
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
    //#[asset(path = "ost.ogg")]
    //ost: Handle<AudioSource>,
    #[asset(path = "testcity.gltf#Scene0")]
    testcity: Handle<Scene>,
    #[asset(path = "anvil.gltf#Scene0")]
    anvil: Handle<Scene>,
}

#[derive(Resource, Default, Debug)]
struct AudioMixer {
    ost: Handle<AudioSink>,
}

fn soundtrack(game_assets: Res<GameAssets>, audio: Res<Audio>, mut mixer: ResMut<AudioMixer>) {
    /*
    mixer.ost = audio.play_with_settings(
        game_assets.ost.clone_weak(),
        PlaybackSettings {
            repeat: true,
            ..default()
        },
    );
    */
}

fn player_ui(mut commands: Commands) {}

fn grab_cursor(mut window_query: Query<&mut Window>) {
    if let Ok(mut window) = window_query.get_single_mut() {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        //window.cursor.visible = false;
        window.cursor.visible = true;
    }
}

fn setup_graphics(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            num_cascades: 4,
            maximum_distance: 1000.0,
            ..default()
        }
        .into(),
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.5, 0.0)),
        ..default()
    });
    commands.spawn(SceneBundle {
        scene: game_assets.testcity.clone(),
        transform: Transform::default().with_scale(Vec3::splat(1.0)),
        ..default()
    });
}

fn add_scene_colliders(
    mut commands: Commands,
    scene_query: Query<(Entity, &SceneInstance)>,
    children: Query<&Children>,
    has_mesh: Query<(&Transform, &Handle<Mesh>)>,
    has_name: Query<&Name>,
    meshes: ResMut<Assets<Mesh>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if scene_query.is_empty() {
        return;
    }
    // very ugly
    if children
        .iter_descendants(scene_query.iter().next().unwrap().0)
        .count()
        < 2
    {
        return;
    }

    for (scene, _) in scene_query.iter() {
        for descendant in children.iter_descendants(scene) {
            if let Ok((_transform, mesh)) = has_mesh.get(descendant) {
                let rapier_collider = Collider::from_bevy_mesh(
                    meshes.get(mesh).unwrap(),
                    &ComputedColliderShape::TriMesh,
                )
                .unwrap();
                // ugly, but will do
                if let Ok(name) = has_name.get(scene) {
                    if name.as_ref() == "Anvil" {
                        commands.entity(scene).insert(rapier_collider);
                    }
                    dbg!(name);
                } else {
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
}

#[derive(Component, Default, Clone, Debug)]
struct PlayerCamera {
    sensitivity: Vec3,
}

#[derive(Component, Default, Clone, Debug)]
struct Anvil {}

#[derive(Component, Default, Clone, Debug)]
struct Held {}

fn spawn_player(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn((
        Anvil::default(),
        Name::new("Anvil"),
        Velocity::default(),
        RigidBody::Dynamic,
        SceneBundle {
            scene: game_assets.anvil.clone(),
            transform: Transform::from_xyz(0.0, 14.0, 0.0),
            ..default()
        },
        ColliderMassProperties::Density(10.0),
        CollisionGroups::new(Group::GROUP_2, Group::ALL),
    ));
    commands
        .spawn((
            Name::new("Player"),
            Player {
                speed: 10.0,
                velocity: Vec3::ZERO,
                jump_strenght: 0.07,
                pickup_distance: 2.0,
            },
            TransformBundle {
                local: Transform::from_xyz(0.0, 14.0, 2.0),
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
        ))
        .with_children(|parent| {
            parent.spawn((
                Camera3dBundle {
                    transform: Transform::from_xyz(0.0, 1.5, 0.0),
                    projection: Projection::Perspective(PerspectiveProjection {
                        fov: 20.0,
                        ..default()
                    }),
                    ..default()
                },
                PlayerCamera {
                    sensitivity: Vec3::new(0.004, 0.004, 1.0),
                },
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
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut cam_query: Query<(&mut PlayerCamera, &mut Transform), Without<Player>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
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
            acceleration *= player.speed;
            if acceleration.length_squared() > 1.0 {
                acceleration = acceleration.normalize()
            }
            acceleration += Vec3::NEG_Y * 0.2;
            player.velocity += acceleration * time.delta_seconds();
            player.velocity.x *= 0.9;
            player.velocity.z *= 0.9;

            contr.translation = Some(player.velocity * time.delta_seconds() * 100.0);
        }
    }
}

fn player_gravity(mut player_query: Query<(&mut Player, &KinematicCharacterControllerOutput)>) {
    if let Ok((mut player, out)) = player_query.get_single_mut() {
        if out.grounded {
            player.velocity.y = 0.0;
        }
    }
}

fn player_jump(
    mut player_query: Query<(&mut Player, &KinematicCharacterControllerOutput)>,
    keys: Res<Input<KeyCode>>,
) {
    if keys.pressed(KeyCode::Space) {
        if let Ok((mut player, out)) = player_query.get_single_mut() {
            if out.grounded {
                player.velocity.y = player.jump_strenght;
            }
        }
    }
}

fn player_pickup(
    mut commands: Commands,
    mut player_query: Query<(&Player, &Transform), Without<Anvil>>,
    mut anvil_query: Query<(Entity, &mut Anvil, &Transform), Without<Player>>,
    held_query: Query<&Held>,
    keys: Res<Input<KeyCode>>,
) {
    match (player_query.get_single_mut(), anvil_query.get_single_mut()) {
        (Ok((player, tr_player)), Ok((anvil_ent, _anvil, tr_anvil))) => {
            if keys.just_pressed(KeyCode::E) {
                if held_query.get(anvil_ent).is_err() {
                    dbg!("ere");
                    let dist = tr_anvil.translation.distance_squared(tr_player.translation);
                    if dist < player.pickup_distance * player.pickup_distance {
                        commands.entity(anvil_ent).insert((
                            Held::default(),
                            RigidBody::Fixed,
                            CollisionGroups::new(Group::GROUP_2, Group::NONE),
                        ));
                    }
                } else {
                    dbg!("ethee");
                    commands
                        .entity(anvil_ent)
                        .insert((
                            RigidBody::Dynamic,
                            Velocity {
                                linvel: player.velocity,
                                ..default()
                            },
                            CollisionGroups::new(Group::GROUP_2, Group::ALL),
                        ))
                        .remove::<Held>();
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
            let off = tr_player.forward() + tr_player.up();
            tr_anvil.translation = tr_player.translation + off;
            tr_anvil.rotation =
                Quat::from_axis_angle(tr_player.up(), tr_player.rotation.to_euler(EulerRot::XYZ).1);
        }
        (_, _) => {}
    }
}
