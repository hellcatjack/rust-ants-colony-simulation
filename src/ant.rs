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

#[derive(Component)]
pub struct Food {
    pub storage: i32,
}

impl Plugin for AntPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .insert_resource(AntScanRadius(INITIAL_ANT_PH_SCAN_RADIUS))
            .insert_resource(AntFollowCameraPos(Vec2::ZERO))
            .add_systems(
                Update,
                (
                    drop_pheromone.run_if(on_timer(Duration::from_secs_f32(ANT_PH_DROP_INTERVAL))),
                    avoid_obstacles,
                    check_wall_collision.after(avoid_obstacles),
                    check_home_food_collisions.run_if(on_timer(Duration::from_secs_f32(0.1))),
                    periodic_direction_update,
                    decay_ph_strength.run_if(on_timer(Duration::from_secs_f32(ANT_PH_STRENGTH_DECAY_INTERVAL))),
                    update_position.after(check_wall_collision),
                    animate_ant,
                ).run_if(run_if_not_paused)
            )
            .add_systems(Update, update_camera_follow_pos)
            .add_systems(
                Update,
                update_stats.run_if(on_timer(Duration::from_secs_f32(3.0))),
            )
            .add_systems(
                Update,
                update_scan_radius.run_if(on_timer(Duration::from_secs_f32(1.0))),
            )
            .add_systems(Update, debug_sensors)
            .add_systems(Update, update_ant_count)
            .add_systems(Update, reset_ants);
    }
}

fn run_if_not_paused(settings: Res<crate::gui::SimSettings>) -> bool {
    !settings.is_paused
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
    config: Res<SimConfig>,
) {
    for _ in events.iter() {
        // Despawn all ants
        for entity in ant_query.iter() {
            commands.entity(entity).despawn();
        }

        // Spawn new ants
        for _ in 0..config.ants_count {
             spawn_ant(&mut commands, &ant_animations.walk);
        }
    }
}

fn update_ant_count(
    mut commands: Commands,
    ant_query: Query<Entity, With<Ant>>,
    config: Res<SimConfig>,
    ant_animations: Res<AntAnimations>,
) {
    if !config.is_changed() {
        return;
    }

    let current_count = ant_query.iter().len();
    let target_count = config.ants_count;

    if current_count < target_count {
        let diff = target_count - current_count;
        for _ in 0..diff {
            spawn_ant(&mut commands, &ant_animations.walk);
        }
    } else if current_count > target_count {
        let diff = current_count - target_count;
        let ants_to_despawn = ant_query.iter().take(diff);
        for entity in ants_to_despawn {
            commands.entity(entity).despawn();
        }
    }
}

fn drop_pheromone(
    mut ant_query: Query<(&Transform, &CurrentTask, &PhStrength), With<Ant>>,
    mut pheromones: ResMut<Pheromones>,
    config: Res<SimConfig>,
) {
    for (transform, ant_task, ph_strength) in ant_query.iter_mut() {
        let x = transform.translation.x as i32;
        let y = transform.translation.y as i32;

        match ant_task.0 {
            AntTask::FindFood => pheromones.to_home.emit_signal(&(x, y), ph_strength.0, config.max_pheromone_strength),
            AntTask::FindHome => pheromones.to_food.emit_signal(&(x, y), ph_strength.0, config.max_pheromone_strength),
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
    _scan_radius: Res<AntScanRadius>,
    config: Res<SimConfig>,
    time: Res<Time>,
    food_query: Query<&Transform, With<Food>>,
    obstacle_map: Res<crate::map::ObstacleMap>,
    map_size: Res<crate::map::MapSize>,
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
                let pull_radius_sq = config.ant_target_auto_pull_radius * config.ant_target_auto_pull_radius;
                let mut best_dist = pull_radius_sq;
                // Find closest food
                for food_transform in food_query.iter() {
                    let food_pos = food_transform.translation.truncate();
                    let dist_sq = transform.translation.distance_squared(food_transform.translation);
                    if dist_sq <= best_dist {
                         // Check Line of Sight
                         if obstacle_map.has_line_of_sight(current_pos.truncate(), food_pos, map_size.width, map_size.height) {
                             best_dist = dist_sq;
                             target = Some(food_pos);
                         }
                    }
                }
            }
            AntTask::FindHome => {
                let home_pos = vec2(HOME_LOCATION.0, HOME_LOCATION.1);
                let dist_to_home = transform.translation.distance_squared(vec3(
                    HOME_LOCATION.0,
                    HOME_LOCATION.1,
                    0.0,
                ));
                if dist_to_home <= config.ant_target_auto_pull_radius * config.ant_target_auto_pull_radius {
                    // Check LOS
                     if obstacle_map.has_line_of_sight(current_pos.truncate(), home_pos, map_size.width, map_size.height) {
                         target = Some(home_pos);
                     }
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
            
            // Sample sensors
            let v_l = grid.sample_sensor_sum(pos_l, ANT_SENSOR_RADIUS);
            let v_r = grid.sample_sensor_sum(pos_r, ANT_SENSOR_RADIUS);
            let v_f = grid.sample_sensor_sum(pos_f, ANT_SENSOR_RADIUS);
            // 3. Normal Steering
            // Use squared values for sharper gradients
            let v_l = v_l.powf(2.0);
            let v_r = v_r.powf(2.0);
            let v_f = v_f.powf(2.0);

            if v_l + v_r + v_f > 0.0 {
                 let dir_l = (pos_l - current_pos.truncate()).normalize();
                 let dir_r = (pos_r - current_pos.truncate()).normalize();
                 let dir_f = (pos_f - current_pos.truncate()).normalize();

                 // Simple Weighted Sum = Forward Bias (due to geometry)
                 let steer_dir = (dir_l * v_l + dir_r * v_r + dir_f * v_f).normalize_or_zero();
                 
                 if steer_dir != Vec2::ZERO {
                      target = Some(current_pos.truncate() + steer_dir * config.ant_sensor_dist);
                 }
            }
        }
 
        if target.is_none() {
            // No signal? Random Search.
            acceleration.0 += get_rand_unit_vec2() * config.ant_turn_randomness;
            continue;
        }

        let steering_force = get_steering_force(
            target.unwrap(),
            transform.translation.truncate(),
            velocity.0,
        );

        let mut rng = rand::thread_rng();
        acceleration.0 += steering_force * rng.gen_range(0.8..=1.2) * config.ant_steering_force_factor;
        // Reduced lateral wiggle on established trails for stability
        acceleration.0 += get_rand_unit_vec2() * (config.ant_turn_randomness * 0.1);
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
    mut commands: Commands,
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
    mut food_query: Query<(Entity, &Transform, &mut Food), Without<Ant>>,
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
            // If we were bringing food home, drop it and turn around
            match ant_task.0 {
                AntTask::FindFood => {
                    // Already looking for food (maybe gave up and came home, or just spawned)
                    // RECHARGE PHEROMONE: Even if empty handed, visiting home refreshes scent supply.
                    ph_strength.0 = ANT_INITIAL_PH_STRENGTH;
                }
                AntTask::FindHome => {
                    // Just arrived home with food.
                    // 1. Drop Food (Switch Task)
                    ant_task.0 = AntTask::FindFood;
                    ph_strength.0 = ANT_INITIAL_PH_STRENGTH;
                    *atlas_handle = ant_animations.walk.clone();
                    sprite.color = Color::rgb(1.0, 1.0, 2.5);

                    // 2. Turn Around to go back to where we came from
                    // Reflect velocity perfectly to head back out the "entrance" we came in
                    velocity.0 *= -1.0; 
                    
                    // Add a tiny bit of noise so they don't walk in a perfect laser line
                     let mut rng = rand::thread_rng();
                     let angle = rng.gen_range(-0.5..0.5); // Small jitter
                     velocity.0 = Vec2::from_angle(angle).rotate(velocity.0);
                }
            }
        }

        // Food Collision
        if let AntTask::FindFood = ant_task.0 {
            for (food_entity, food_transform, mut food) in food_query.iter_mut() {
                let dist_to_food = transform.translation.distance_squared(food_transform.translation);
                
                if dist_to_food < FOOD_PICKUP_RADIUS * FOOD_PICKUP_RADIUS {
                     // Collided with food
                     velocity.0 *= -1.0;
                     ant_task.0 = AntTask::FindHome;
                     ph_strength.0 = ANT_INITIAL_PH_STRENGTH;
                     
                     *atlas_handle = ant_animations.walk_food.clone();
                     sprite.color = Color::rgb(1.0, 2.0, 1.0);
                     
                     food.storage -= 1;
                     if food.storage <= 0 {
                         commands.entity(food_entity).despawn();
                     }
                     
                     // Stop checking other foods for this ant
                     break; 
                }
            }
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
    mut ant_query: Query<(&mut Transform, &mut Velocity, &mut Acceleration), With<Ant>>,
    obstacle_map: Res<crate::map::ObstacleMap>,
    map_size: Res<crate::map::MapSize>,
) {
    let w = map_size.width;
    let h = map_size.height;

    for (mut transform, mut velocity, mut acceleration) in ant_query.iter_mut() {
        // wall rebound
        let border = 20.0;
        let top_left = (-w / 2.0, h / 2.0);
        let bottom_right = (w / 2.0, -h / 2.0);
        
        let pos = transform.translation;
        
        let mut hit_wall = false;
        
        // Wall Clamping
        if pos.x < top_left.0 + border {
            transform.translation.x = top_left.0 + border;
            hit_wall = true;
        } else if pos.x >= bottom_right.0 - border {
            transform.translation.x = bottom_right.0 - border;
            hit_wall = true;
        }
        
        if pos.y > top_left.1 - border {
            transform.translation.y = top_left.1 - border;
            hit_wall = true;
        } else if pos.y < bottom_right.1 + border {
            transform.translation.y = bottom_right.1 + border;
            hit_wall = true;
        }

        let mut hit_obstacle = false;
        if !hit_wall {
             // Check obstacle map with radius
             // Radius reduced to 10.0 for tighter visual collision
             if obstacle_map.is_obstacle_in_radius(pos.x, pos.y, 10.0, w, h) {
                 hit_obstacle = true;
                 
                 // Push ant back slightly to unstuck (opposite to current velocity)
                 let push_dir = -velocity.0.normalize_or_zero();
                 transform.translation.x += push_dir.x * 2.0;
                 transform.translation.y += push_dir.y * 2.0;
             }
        }

        if hit_wall || hit_obstacle {
            // "Stop and observe" behavior
            // Heavily dampen velocity and reverse it slightly to detach from wall
            velocity.0 = -velocity.0 * 0.2;
            
            // Clear acceleration
            acceleration.0 = Vec2::ZERO;

            // Add a small random rotation to velocity to simulate "looking for new direction"
            let mut rng = thread_rng();
            let jitter_angle: f32 = rng.gen_range(-1.0..1.0); 
            let cos_a = jitter_angle.cos();
            let sin_a = jitter_angle.sin();
            velocity.0 = vec2(
                velocity.0.x * cos_a - velocity.0.y * sin_a,
                velocity.0.x * sin_a + velocity.0.y * cos_a
            );
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

fn avoid_obstacles(
    mut ant_query: Query<(&Transform, &Velocity, &mut Acceleration), With<Ant>>,
    obstacle_map: Res<crate::map::ObstacleMap>,
    map_size: Res<crate::map::MapSize>,
) {
    let w = map_size.width;
    let h = map_size.height;
    // Look ahead distance reduced to 20.0 for closer reaction
    let look_ahead = 20.0;
    // Sensor probe offset angle (radians)
    let probe_angle: f32 = 0.5; // ~30 degrees

    for (transform, velocity, mut acceleration) in ant_query.iter_mut() {
        if velocity.0.length_squared() < 0.1 { continue; }
        
        let forward = velocity.0.normalize();
        let pos = transform.translation.truncate();
        
        // Center probe
        let center_probe = pos + forward * look_ahead;
        
        // Left probe
        let cos_a = probe_angle.cos();
        let sin_a = probe_angle.sin();
        let left_dir = vec2(
            forward.x * cos_a - forward.y * sin_a,
            forward.x * sin_a + forward.y * cos_a
        );
        let left_probe = pos + left_dir * look_ahead;
        
        // Right probe
        let right_dir = vec2(
            forward.x * cos_a + forward.y * sin_a,
            -forward.x * sin_a + forward.y * cos_a
        );
        let right_probe = pos + right_dir * look_ahead;
        
        let center_hit = obstacle_map.is_obstacle_in_radius(center_probe.x, center_probe.y, 5.0, w, h);
        let left_hit = obstacle_map.is_obstacle_in_radius(left_probe.x, left_probe.y, 5.0, w, h);
        let right_hit = obstacle_map.is_obstacle_in_radius(right_probe.x, right_probe.y, 5.0, w, h);
        
        if center_hit || left_hit || right_hit {
            let mut turn_force = Vec2::ZERO;
            
            // Steering logic
            
            if left_hit && !right_hit {
                // Obstacle on left, turn right
                turn_force += vec2(forward.y, -forward.x) * 400.0;
            } else if right_hit && !left_hit {
                // Obstacle on right, turn left
                turn_force += vec2(-forward.y, forward.x) * 400.0;
            } else {
                // Both blocked or center blocked, pick random valid side or turn around
                let mut rng = thread_rng();
                if rng.gen_bool(0.5) {
                    turn_force += vec2(forward.y, -forward.x) * 600.0;
                } else {
                    turn_force += vec2(-forward.y, forward.x) * 600.0;
                }
            }
            
            if center_hit {
                 // Brake hard
                 acceleration.0 -= velocity.0 * 2.0;
            }

            acceleration.0 += turn_force;
        }
    }
}
