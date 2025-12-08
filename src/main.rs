use ants::{
    ant::{AntFollowCameraPos, AntPlugin},
    gui::{GuiPlugin, SimSettings},
    pathviz::PathVizPlugin,
    pheromone::PheromonePlugin,
    map::MapPlugin,
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
        .add_systems(Update, update_border_size)
        .add_systems(Last, limit_fps)
        // Internal Plugins
        .add_plugins(AntPlugin)
        .add_plugins(PheromonePlugin)
        .add_plugins(PathVizPlugin)
        .add_plugins(MapPlugin)
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
    transform.translation = vec3(ant_pos.0.x, ant_pos.0.y, ANT_Z_INDEX + 500.0);
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    commands
        .spawn((
            Camera2dBundle {
                camera: Camera {
                    hdr: true,
                    ..default()
                },
                tonemapping: Tonemapping::TonyMcMapface,
                transform: Transform::from_xyz(0.0, 0.0, 500.0),
                ..default()
            },
            BloomSettings::default(),
            FollowCamera,
        ))
        .insert(PanCam {
            grab_buttons: vec![MouseButton::Right, MouseButton::Middle],
            ..default()
        });

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

    // Programmatic Border (Glass Tank Effect)
    // Create a 1x1 White Pixel Texture
    let image = Image::new_fill(
        bevy::render::render_resource::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        &[255, 255, 255, 255],
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
    );
    let handle = images.add(image);
    let color = Color::rgba(0.5, 0.8, 1.0, 0.5); // Cyan Glass
    
    // Spawn 4 segments with placeholder transforms (updated by system immediately)
    let segments = vec![BorderSegment::Top, BorderSegment::Bottom, BorderSegment::Left, BorderSegment::Right];
    for segment in segments {
        commands.spawn((
            SpriteBundle {
                texture: handle.clone(),
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::new(100.0, 100.0)), // Initial
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 20.0), 
                ..default()
            },
            MapBorder,
            segment,
        ));
    }
}

#[derive(Component)]
struct MapBorder;

#[derive(Component)]
enum BorderSegment {
    Top, Bottom, Left, Right
}

fn update_border_size(
    map_size: Res<ants::map::MapSize>,
    mut query: Query<(&mut Sprite, &mut Transform, &BorderSegment), With<MapBorder>>,
) {
    if map_size.is_changed() {
        let w = map_size.width;
        let h = map_size.height;
        let t = 20.0; // Thickness
        
        for (mut sprite, mut transform, segment) in query.iter_mut() {
             match segment {
                 BorderSegment::Top => {
                     sprite.custom_size = Some(Vec2::new(w + 2.0 * t, t));
                     transform.translation.y = h / 2.0 + t / 2.0;
                     transform.translation.x = 0.0;
                 },
                 BorderSegment::Bottom => {
                     sprite.custom_size = Some(Vec2::new(w + 2.0 * t, t));
                     transform.translation.y = -(h / 2.0 + t / 2.0);
                     transform.translation.x = 0.0;
                 },
                 BorderSegment::Left => {
                     sprite.custom_size = Some(Vec2::new(t, h));
                     transform.translation.x = -(w / 2.0 + t / 2.0);
                     transform.translation.y = 0.0;
                 },
                 BorderSegment::Right => {
                     sprite.custom_size = Some(Vec2::new(t, h));
                     transform.translation.x = w / 2.0 + t / 2.0;
                     transform.translation.y = 0.0;
                 },
             }
        }
    }
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
