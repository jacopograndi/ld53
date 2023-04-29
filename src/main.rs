use bevy::{
    input::mouse::MouseMotion, pbr::CascadeShadowConfigBuilder, prelude::*, scene::SceneInstance,
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
            (setup_graphics, spawn_player, player_ui).in_schedule(OnEnter(GameState::PrepareScene)),
        )
        .add_system(add_scene_colliders.in_set(OnUpdate(GameState::PrepareScene)))
        .add_system(soundtrack.in_schedule(OnEnter(GameState::Play)))
        .add_systems(
            (movement, camera_view, hands)
                .chain()
                .in_set(OnUpdate(GameState::Play)),
        )
        .init_resource::<AudioMixer>()
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

fn setup_graphics(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        // This is a relatively small scene, so use tighter shadow
        // cascade bounds than the default for better quality.
        // We also adjusted the shadow map to be larger since we're
        // only using a single cascade.
        cascade_shadow_config: CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 1.6,
            ..default()
        }
        .into(),
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
    scene_query: Query<Entity, &SceneInstance>,
    children: Query<&Children>,
    has_mesh: Query<(&Transform, &Handle<Mesh>)>,
    meshes: ResMut<Assets<Mesh>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if scene_query.is_empty() {
        return;
    }
    if children
        .iter_descendants(scene_query.iter().next().unwrap())
        .next()
        == None
    {
        return;
    }

    for scene in &scene_query {
        for descendant in children.iter_descendants(scene) {
            if let Ok((_transform, mesh)) = has_mesh.get(descendant) {
                let rapier_collider = Collider::from_bevy_mesh(
                    meshes.get(mesh).unwrap(),
                    &ComputedColliderShape::TriMesh,
                )
                .unwrap();
                commands.entity(descendant).insert(rapier_collider);
            }
        }
    }
    next_state.set(GameState::Play);
}

#[derive(Component, Default, Clone, Debug)]
struct Player {
    speed: f32,
}

#[derive(Component, Default, Clone, Debug)]
struct PlayerCamera {
    sensitivity: Vec3,
}

fn spawn_player(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 2.0 })),
                transform: Transform::from_xyz(0.0, 14.0, 2.0),
                ..default()
            },
            Player { speed: 100.0 },
            Velocity::default(),
            RigidBody::Dynamic,
            Collider::capsule(Vec3::new(0.0, 0.25, 0.0), Vec3::new(0.0, 1.5, 0.0), 0.25),
        ))
        .with_children(|parent| {
            parent.spawn((
                Camera3dBundle {
                    transform: Transform::from_xyz(0.0, 1.5, 0.0),
                    ..default()
                },
                PlayerCamera {
                    sensitivity: Vec3::new(0.04, 0.04, 1.0),
                },
            ));
        });
}

fn movement(
    mut player_query: Query<(&mut Player, &mut Transform, &mut Velocity)>,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    if let Ok((player, tr, mut vel)) = player_query.get_single_mut() {
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
        let norm = if acceleration.length_squared() > 1.0 {
            acceleration.normalize()
        } else {
            acceleration
        };
        vel.linvel += norm * player.speed * time.delta_seconds();
    }
}

fn camera_view(
    mut cam_query: Query<(&mut PlayerCamera, &mut Transform)>,
    mut mouse_motion_events: EventReader<MouseMotion>,
) {
    if let Ok((cam, mut tr)) = cam_query.get_single_mut() {
        for mov in mouse_motion_events.iter() {
            tr.rotate_local_x(-mov.delta.y * cam.sensitivity.y);
            tr.rotate_y(-mov.delta.x * cam.sensitivity.y);
        }
    }
}

fn hands(
    mut player_query: Query<(&mut Player, &mut Transform, &mut Velocity)>,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
}
