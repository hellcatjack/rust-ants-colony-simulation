use ants::{
    ant::{AntFollowCameraPos, AntPlugin},
    gui::{GuiPlugin, SimSettings},
    pathviz::PathVizPlugin,
    pheromone::PheromonePlugin,
    *,
};
use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::vec3,
    prelude::*,
};
use bevy_pancam::{PanCam, PanCamPlugin};
use std::{thread, time::{Duration, Instant}};
#[derive(Component)]
struct FollowCamera;

#[derive(Resource)]
struct FrameLimiter {
    last_frame: Instant,
    target_fps: Option<u32>,
}

impl Default for FrameLimiter {
    fn default() -> Self {
        Self {
            last_frame: Instant::now(),
            target_fps: Some(60),
        }
    }
}

fn main() {
    App::new()
        .init_resource::<FrameLimiter>()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resizable: false,
                        // mode: WindowMode::Fullscreen,
                        focused: true,
                        resolution: (W, H).into(),
                        title: "Ants".to_string(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        // External plugins & systems
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_systems(Update, bevy::window::close_on_esc)
        .add_plugins(PanCamPlugin)
        // Default Resources
        .insert_resource(ClearColor(Color::rgba_u8(
            BG_COLOR.0, BG_COLOR.1, BG_COLOR.2, 0,
        )))
        .insert_resource(Msaa::Off)
        // Systems
        .add_systems(Startup, setup)
        .add_systems(Update, ant_follow_camera)
        .add_systems(Last, limit_fps)
        // Internal Plugins
        .add_plugins(AntPlugin)
        .add_plugins(PheromonePlugin)
        .add_plugins(PathVizPlugin)
        .add_plugins(GuiPlugin)
        .run();
}

fn ant_follow_camera(
    ant_pos: Res<AntFollowCameraPos>,
    sim_settings: Res<SimSettings>,
    mut camera_query: Query<&mut Transform, With<FollowCamera>>,
) {
    if !sim_settings.is_camera_follow {
        return;
    }

    let mut transform = camera_query.single_mut();
    transform.translation = vec3(ant_pos.0.x, ant_pos.0.y, ANT_Z_INDEX);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Camera2dBundle {
                camera: Camera {
                    hdr: true,
                    ..default()
                },
                tonemapping: Tonemapping::TonyMcMapface,
                ..default()
            },
            BloomSettings::default(),
            FollowCamera,
        ))
        .insert(PanCam::default());

    // Ant colony sprite
    commands.spawn(SpriteBundle {
        texture: asset_server.load(SPRITE_ANT_COLONY),
        sprite: Sprite {
            color: Color::rgb(1.5, 1.5, 1.5),
            ..default()
        },
        transform: Transform::from_xyz(HOME_LOCATION.0, HOME_LOCATION.1, 2.0)
            .with_scale(Vec3::splat(HOME_SPRITE_SCALE)),
        ..Default::default()
    });

    // Food sprite
    commands.spawn(SpriteBundle {
        texture: asset_server.load(SPRITE_FOOD),
        sprite: Sprite {
            color: Color::rgb(1.5, 1.5, 1.5),
            ..default()
        },
        transform: Transform::from_xyz(FOOD_LOCATION.0, FOOD_LOCATION.1, 2.0)
            .with_scale(Vec3::splat(FOOD_SPRITE_SCALE)),
        ..Default::default()
    });
}

fn limit_fps(mut limiter: ResMut<FrameLimiter>, keys: Res<Input<KeyCode>>) {
    if keys.just_pressed(KeyCode::Minus) {
        if limiter.target_fps.is_none() {
            limiter.target_fps = Some(60);
            println!("Speed: Normal (60 FPS)");
        } else if limiter.target_fps == Some(60) {
            limiter.target_fps = Some(30);
            println!("Speed: Slow (30 FPS)");
        }
    }
    if keys.just_pressed(KeyCode::Equals) {
        if limiter.target_fps == Some(30) {
            limiter.target_fps = Some(60);
            println!("Speed: Normal (60 FPS)");
        } else if limiter.target_fps == Some(60) {
            limiter.target_fps = None;
            println!("Speed: Fast (Unlimited)");
        }
    }

    if let Some(target_fps) = limiter.target_fps {
        if target_fps > 0 {
            let target_duration = Duration::from_secs_f32(1.0 / target_fps as f32);
            let elapsed = limiter.last_frame.elapsed();
            if elapsed < target_duration {
                thread::sleep(target_duration - elapsed);
            }
        }
    }
    limiter.last_frame = Instant::now();
}
