use crate::{
    gui::{ResetSimEvent, SimConfig, SimStatistics},
    pheromone::Pheromones,
    utils::{calc_rotation_angle, get_rand_unit_vec2},
    *,
};
use bevy::{
    math::{vec2, vec3},
    prelude::*,
    time::common_conditions::on_timer,
};
use rand::{thread_rng, Rng};
use std::{f32::consts::PI, time::Duration};

pub struct AntPlugin;

pub enum AntTask {
    FindFood,
    FindHome,
}

#[derive(Component)]
pub struct Ant;
#[derive(Component)]
pub struct CurrentTask(pub AntTask);
#[derive(Component)]
struct Velocity(Vec2);
#[derive(Component)]
struct Acceleration(Vec2);
#[derive(Component)]
struct PhStrength(f32);

#[derive(Resource)]
struct AntScanRadius(f32);
#[derive(Resource)]
pub struct AntFollowCameraPos(pub Vec2);

#[derive(Component)]
struct AnimationTimer(Timer);

#[derive(Resource)]
pub struct AntAnimations {
    pub walk: Handle<TextureAtlas>,
    pub walk_food: Handle<TextureAtlas>,
}

#[derive(Component)]
pub struct DecisionTimer(pub f32);

impl Plugin for AntPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .insert_resource(AntScanRadius(INITIAL_ANT_PH_SCAN_RADIUS))
            .insert_resource(AntFollowCameraPos(Vec2::ZERO))
            .add_systems(
                Update,
                drop_pheromone.run_if(on_timer(Duration::from_secs_f32(ANT_PH_DROP_INTERVAL))),
            )
            .add_systems(
                Update,
                check_wall_collision.run_if(on_timer(Duration::from_secs_f32(0.1))),
            )
            .add_systems(
                Update,
                check_home_food_collisions.run_if(on_timer(Duration::from_secs_f32(0.1))),
            )
            .add_systems(Update, update_camera_follow_pos)
            .add_systems(Update, periodic_direction_update)
            .add_systems(
                Update,
                update_stats.run_if(on_timer(Duration::from_secs_f32(3.0))),
            )
            .add_systems(
                Update,
                update_scan_radius.run_if(on_timer(Duration::from_secs_f32(1.0))),
            )
            .add_systems(
                Update,
                decay_ph_strength.run_if(on_timer(Duration::from_secs_f32(
                    ANT_PH_STRENGTH_DECAY_INTERVAL,
                ))),
            )
            .add_systems(Update, update_position.after(check_wall_collision))
            .add_systems(Update, animate_ant)
            .add_systems(Update, debug_sensors)
            .add_systems(Update, reset_ants);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_handle = asset_server.load(SPRITE_ANT_SHEET);
    let texture_atlas = TextureAtlas::from_grid(
        texture_handle,
        Vec2::new(512.0, 512.0),
        ANT_SHEET_COLS,
        ANT_SHEET_ROWS,
        None,
        None,
    );
    let walk_handle = texture_atlases.add(texture_atlas);

    let texture_handle_food = asset_server.load(SPRITE_ANT_WITH_FOOD_SHEET);
    let texture_atlas_food = TextureAtlas::from_grid(
        texture_handle_food,
        Vec2::new(512.0, 512.0),
        ANT_SHEET_COLS,
        ANT_SHEET_ROWS,
        None,
        None,
    );
    let walk_food_handle = texture_atlases.add(texture_atlas_food);

    commands.insert_resource(AntAnimations {
        walk: walk_handle.clone(),
        walk_food: walk_food_handle,
    });

    for _ in 0..NUM_ANTS {
        spawn_ant(&mut commands, &walk_handle);
    }
}

fn spawn_ant(commands: &mut Commands, texture: &Handle<TextureAtlas>) {
    commands.spawn((
        SpriteSheetBundle {
            texture_atlas: texture.clone(),
            sprite: TextureAtlasSprite::new(0),
            transform: Transform::from_xyz(HOME_LOCATION.0, HOME_LOCATION.1, ANT_Z_INDEX)
                .with_scale(Vec3::splat(ANT_SPRITE_SCALE)),
            ..Default::default()
        },
        Ant,
        CurrentTask(AntTask::FindFood),
        Velocity(get_rand_unit_vec2()),
        Acceleration(Vec2::ZERO),
        PhStrength(ANT_INITIAL_PH_STRENGTH),
        AnimationTimer(Timer::from_seconds(ANT_ANIMATION_SPEED, TimerMode::Repeating)),
        DecisionTimer(thread_rng().gen_range(0.0..0.1)),
    ));
}

fn reset_ants(
    mut commands: Commands,
    mut events: EventReader<ResetSimEvent>,
    ant_query: Query<Entity, With<Ant>>,
    ant_animations: Res<AntAnimations>,
) {
    for _ in events.iter() {
        // Despawn all ants
        for entity in ant_query.iter() {
            commands.entity(entity).despawn();
        }

        // Spawn new ants
        for _ in 0..NUM_ANTS {
             spawn_ant(&mut commands, &ant_animations.walk);
        }
    }
}

fn drop_pheromone(
    mut ant_query: Query<(&Transform, &CurrentTask, &PhStrength), With<Ant>>,
    mut pheromones: ResMut<Pheromones>,
) {
    for (transform, ant_task, ph_strength) in ant_query.iter_mut() {
        let x = transform.translation.x as i32;
        let y = transform.translation.y as i32;

        match ant_task.0 {
            AntTask::FindFood => pheromones.to_home.emit_signal(&(x, y), ph_strength.0),
            AntTask::FindHome => pheromones.to_food.emit_signal(&(x, y), ph_strength.0),
        }
    }
}

fn update_scan_radius(mut scan_radius: ResMut<AntScanRadius>) {
    if scan_radius.0 > INITIAL_ANT_PH_SCAN_RADIUS * ANT_PH_SCAN_RADIUS_SCALE {
        return;
    }

    scan_radius.0 += ANT_PH_SCAN_RADIUS_INCREMENT;
}

fn update_camera_follow_pos(
    ant_query: Query<&Transform, With<Ant>>,
    mut follow_pos: ResMut<AntFollowCameraPos>,
) {
    if let Some(transform) = ant_query.iter().next() {
        follow_pos.0 = transform.translation.truncate();
    }
}

fn update_stats(
    mut stats: ResMut<SimStatistics>,
    scan_radius: Res<AntScanRadius>,
    ant_query: Query<With<Ant>>,
) {
    stats.scan_radius = scan_radius.0;
    stats.num_ants = ant_query.iter().len();
}

fn decay_ph_strength(mut ant_query: Query<&mut PhStrength, With<Ant>>, config: Res<SimConfig>) {
    for mut ph_strength in ant_query.iter_mut() {
        ph_strength.0 = f32::max(ph_strength.0 - config.ant_ph_strength_decay_rate, 0.0);
    }
}

fn get_steering_force(target: Vec2, current: Vec2, velocity: Vec2) -> Vec2 {
    let desired = target - current;
    let steering = desired - velocity;
    steering * 0.2
}

fn periodic_direction_update(
    mut ant_query: Query<(&mut Acceleration, &Transform, &CurrentTask, &Velocity, &mut DecisionTimer), With<Ant>>,
    mut pheromones: ResMut<Pheromones>,
    mut stats: ResMut<SimStatistics>,
    scan_radius: Res<AntScanRadius>,
    config: Res<SimConfig>,
    time: Res<Time>,
) {
    (stats.food_cache_size, stats.home_cache_size) = pheromones.clear_cache();

    for (mut acceleration, transform, current_task, velocity, mut timer) in ant_query.iter_mut() {
        timer.0 -= time.delta_seconds();
        if timer.0 > 0.0 {
            continue;
        }
        // Reset timer with some randomness to prevent syncing
        timer.0 = config.ant_update_interval + thread_rng().gen_range(-0.01..0.01);

        let current_pos = transform.translation;
        let mut target = None;

        // If ant is close to food/home, pull it towards itself
        match current_task.0 {
            AntTask::FindFood => {
                let dist_to_food = transform.translation.distance_squared(vec3(
                    FOOD_LOCATION.0,
                    FOOD_LOCATION.1,
                    0.0,
                ));
                if dist_to_food <= ANT_TARGET_AUTO_PULL_RADIUS * ANT_TARGET_AUTO_PULL_RADIUS {
                    target = Some(vec2(FOOD_LOCATION.0, FOOD_LOCATION.1));
                }
            }
            AntTask::FindHome => {
                let dist_to_home = transform.translation.distance_squared(vec3(
                    HOME_LOCATION.0,
                    HOME_LOCATION.1,
                    0.0,
                ));
                if dist_to_home <= ANT_TARGET_AUTO_PULL_RADIUS * ANT_TARGET_AUTO_PULL_RADIUS {
                    target = Some(vec2(HOME_LOCATION.0, HOME_LOCATION.1));
                }
            }
        };

        if target.is_none() {


            // Sensor Based Steering
            // 1. Calculate Sensor Positions
            let (pos_l, pos_r, pos_f) = calculate_sensor_positions(
                current_pos.truncate(), 
                velocity.0,
                config.ant_sensor_dist,
                config.ant_sensor_angle,
            );
            
            // 2. Sample Strength
             // Optimization: We know which grid we need.
            let grid = match current_task.0 {
                AntTask::FindFood => &pheromones.to_food,
                AntTask::FindHome => &pheromones.to_home,
            };
            
            let v_l = grid.sample_sensor_sum(pos_l, ANT_SENSOR_RADIUS);
            let v_r = grid.sample_sensor_sum(pos_r, ANT_SENSOR_RADIUS);
            let v_f = grid.sample_sensor_sum(pos_f, ANT_SENSOR_RADIUS);
            
            // 3. Decide Target
            if v_l + v_r + v_f > 0.0 {
                 // Re-derive directions for weighting (normalized)
                 let velocity_dir = velocity.0.normalize_or_zero();
                 let forward = if velocity_dir == Vec2::ZERO { vec2(1.0, 0.0) } else { velocity_dir };
                 
                 // We need the directions again to sum them. 
                 // Since we refactored positions, let's just use (Pos - Current).normalize()
                 let dir_l = (pos_l - current_pos.truncate()).normalize();
                 let dir_r = (pos_r - current_pos.truncate()).normalize();
                 let dir_f = (pos_f - current_pos.truncate()).normalize();

                 let steer_dir = (dir_l * v_l + dir_r * v_r + dir_f * v_f).normalize_or_zero();
                 
                 // If resulting vec is valid, set target
                 if steer_dir != Vec2::ZERO {
                      target = Some(current_pos.truncate() + steer_dir * config.ant_sensor_dist);
                 }
            }
        }

        if target.is_none() {
            // Default direction randomization
            acceleration.0 += get_rand_unit_vec2() * config.ant_turn_randomness;
            continue;
        }

        let steering_force = get_steering_force(
            target.unwrap(),
            transform.translation.truncate(),
            velocity.0,
        );

        let mut rng = rand::thread_rng();
        acceleration.0 += steering_force * rng.gen_range(0.5..=1.0) * ANT_STEERING_FORCE_FACTOR;
        // Add wiggle while following trail
        // Scale wiggle down based on global randomness setting, but keep it proportional
        acceleration.0 += get_rand_unit_vec2() * (config.ant_turn_randomness * 0.33); 
    }
}

fn calculate_sensor_positions(
    current_pos: Vec2, 
    velocity: Vec2,
    sensor_dist: f32,
    sensor_angle: f32,
) -> (Vec2, Vec2, Vec2) {
    let velocity_dir = velocity.normalize_or_zero();
    let forward = if velocity_dir == Vec2::ZERO {
            vec2(1.0, 0.0) 
    } else { 
            velocity_dir 
    };
    
    let angle_rad = sensor_angle.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    
    // Left (Rotate +Angle)
    let left_dir = vec2(
        forward.x * cos_a - forward.y * sin_a,
        forward.x * sin_a + forward.y * cos_a
    );
    
    // Right (Rotate -Angle)
    let right_dir = vec2(
        forward.x * cos_a - forward.y * (-sin_a),
        forward.x * (-sin_a) + forward.y * cos_a
    );
    
    let pos_l = current_pos + left_dir * sensor_dist;
    let pos_r = current_pos + right_dir * sensor_dist;
    let pos_f = current_pos + forward * sensor_dist;

    (pos_l, pos_r, pos_f)
}

fn debug_sensors(
    mut gizmos: Gizmos,
    ant_query: Query<(&Transform, &Velocity), With<Ant>>,
    settings: Res<crate::gui::SimSettings>,
    config: Res<SimConfig>,
) {
    if !settings.is_show_ants_path && !settings.is_show_sensor_radius {
        return;
    }

    for (transform, velocity) in ant_query.iter() {
        let pos = transform.translation.truncate();
        
        if settings.is_show_ants_path {
            let (l, r, f) = calculate_sensor_positions(
                pos, 
                velocity.0,
                config.ant_sensor_dist,
                config.ant_sensor_angle
            );
            
            gizmos.line_2d(pos, l, Color::RED);   // Left
            gizmos.line_2d(pos, r, Color::BLUE);  // Right
            gizmos.line_2d(pos, f, Color::GREEN); // Front;
        }

        if settings.is_show_sensor_radius {
             gizmos.circle_2d(pos, config.ant_sensor_dist, Color::rgba(0.0, 1.0, 1.0, 0.1));
        }
    }
}

fn check_home_food_collisions(
    mut ant_query: Query<
        (
            &Transform,
            &mut TextureAtlasSprite,
            &mut Velocity,
            &mut CurrentTask,
            &mut PhStrength,
            &mut Handle<TextureAtlas>,
        ),
        With<Ant>,
    >,
    ant_animations: Res<AntAnimations>,
) {
    for (transform, mut sprite, mut velocity, mut ant_task, mut ph_strength, mut atlas_handle) in
        ant_query.iter_mut()
    {
        // Home collision
        let dist_to_home =
            transform
                .translation
                .distance_squared(vec3(HOME_LOCATION.0, HOME_LOCATION.1, 0.0));
        if dist_to_home < HOME_RADIUS * HOME_RADIUS {
            // rebound only the ants with food
            match ant_task.0 {
                AntTask::FindFood => {}
                AntTask::FindHome => {
                    velocity.0 *= -1.0;
                }
            }
            ant_task.0 = AntTask::FindFood;
            ph_strength.0 = ANT_INITIAL_PH_STRENGTH;
            *atlas_handle = ant_animations.walk.clone();
            sprite.color = Color::rgb(1.0, 1.0, 2.5);
        }

        // Food Collision
        let dist_to_food =
            transform
                .translation
                .distance_squared(vec3(FOOD_LOCATION.0, FOOD_LOCATION.1, 0.0));
        if dist_to_food < FOOD_PICKUP_RADIUS * FOOD_PICKUP_RADIUS {
            match ant_task.0 {
                AntTask::FindFood => {
                    velocity.0 *= -1.0;
                }
                AntTask::FindHome => {}
            }
            ant_task.0 = AntTask::FindHome;
            ph_strength.0 = ANT_INITIAL_PH_STRENGTH;
            
            println!("Collision! Swapping texture to Food Handle: {:?}", ant_animations.walk_food);
            *atlas_handle = ant_animations.walk_food.clone();
            println!("Swap success.");
            
            sprite.color = Color::rgb(1.0, 2.0, 1.0);
        }
    }
}

fn animate_ant(
    time: Res<Time>,
    mut query: Query<(&mut AnimationTimer, &mut TextureAtlasSprite), With<Ant>>,
) {
    for (mut timer, mut sprite) in &mut query {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            sprite.index = (sprite.index + 1) % (ANT_SHEET_COLS * ANT_SHEET_ROWS);
        }
    }
}

fn check_wall_collision(
    mut ant_query: Query<(&Transform, &Velocity, &mut Acceleration), With<Ant>>,
) {
    for (transform, velocity, mut acceleration) in ant_query.iter_mut() {
        // wall rebound
        let border = 20.0;
        let top_left = (-W / 2.0, H / 2.0);
        let bottom_right = (W / 2.0, -H / 2.0);
        let x_bound = transform.translation.x < top_left.0 + border
            || transform.translation.x >= bottom_right.0 - border;
        let y_bound = transform.translation.y >= top_left.1 - border
            || transform.translation.y < bottom_right.1 + border;
        if x_bound || y_bound {
            let mut rng = thread_rng();
            let target = vec2(rng.gen_range(-200.0..200.0), rng.gen_range(-200.0..200.0));
            acceleration.0 +=
                get_steering_force(target, transform.translation.truncate(), velocity.0);
        }
    }
}

fn update_position(
    mut ant_query: Query<(&mut Transform, &mut Velocity, &mut Acceleration), With<Ant>>,
) {
    for (mut transform, mut velocity, mut acceleration) in ant_query.iter_mut() {
        let old_pos = transform.translation;

        if !acceleration.0.is_nan() {
            velocity.0 = (velocity.0 + acceleration.0).normalize();
            let new_translation =
                transform.translation + vec3(velocity.0.x, velocity.0.y, 0.0) * ANT_SPEED;
            if !new_translation.is_nan() {
                transform.translation = new_translation;
            }
        }

        acceleration.0 = Vec2::ZERO;
        transform.rotation =
            Quat::from_rotation_z(calc_rotation_angle(old_pos, transform.translation) + PI);
    }
}
