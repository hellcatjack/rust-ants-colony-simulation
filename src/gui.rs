use crate::{ant::{Ant, Food}, *};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_pancam::PanCam;
use crate::map::{MapSize, ObstacleMap};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum EditorTool {
    None,
    BrushObstacle,
    EraserObstacle,
    PlaceFood,
    RemoveFood,
}

#[derive(Resource)]
pub struct EditorState {
    pub selected_tool: EditorTool,
    pub brush_size: f32,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selected_tool: EditorTool::None,
            brush_size: 20.0,
        }
    }
}

pub struct GuiPlugin;

#[derive(Resource)]
pub struct SimSettings {
    pub is_show_home_ph: bool,
    pub is_show_food_ph: bool,
    pub is_show_ants: bool,
    pub is_camera_follow: bool,
    pub is_show_menu: bool,
    pub is_show_ants_path: bool,
    pub is_show_sensor_radius: bool,
    pub is_paused: bool,
}

impl Default for SimSettings {
    fn default() -> Self {
        Self {
            is_show_home_ph: true,
            is_show_food_ph: true,
            is_show_ants: true,
            is_camera_follow: false,
            is_show_menu: true,
            is_show_ants_path: false,
            is_show_sensor_radius: false,
            is_paused: false,
        }
    }
}
impl Plugin for GuiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(SimSettings::default())
            .insert_resource(SimStatistics::default())
            .insert_resource(SimConfig::default())
            .insert_resource(EditorState::default())
            .add_event::<ResetSimEvent>()
            .add_systems(Update, settings_dialog)
            .add_systems(Update, settings_toggle)
            .add_systems(Update, handle_camera_control)
            .add_systems(Update, editor_ui)
            .add_systems(Update, handle_editor_input)
            .add_plugins(EguiPlugin)
            .add_systems(Startup, (setup, configure_ui, load_config));
    }
}
// ...

fn settings_toggle(
    mut settings: ResMut<SimSettings>,
    mut ant_query: Query<&mut Visibility, With<Ant>>,
    keys: Res<Input<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        settings.is_show_menu = !settings.is_show_menu;
    }
    if keys.just_pressed(KeyCode::Space) {
        settings.is_paused = !settings.is_paused;
        println!("Paused: {}", settings.is_paused);
    }
    if keys.just_pressed(KeyCode::H) {
        settings.is_show_home_ph = !settings.is_show_home_ph;
    }
    if keys.just_pressed(KeyCode::F) {
        settings.is_show_food_ph = !settings.is_show_food_ph;
    }
    if keys.just_pressed(KeyCode::P) {
        settings.is_show_ants_path = !settings.is_show_ants_path;
        settings.is_show_sensor_radius = !settings.is_show_sensor_radius;
    }
    if keys.just_pressed(KeyCode::A) {
        settings.is_show_ants = !settings.is_show_ants;
        toggle_ant_visibility(ant_query, settings.is_show_ants);
    }
}

#[derive(Event)]
pub struct ResetSimEvent;

#[derive(Resource, Serialize, Deserialize, Clone)]
pub struct SimConfig {
    pub ph_decay_rate: f32,
    pub ant_ph_strength_decay_rate: f32,
    pub ant_sensor_dist: f32,
    pub ant_sensor_angle: f32,
    pub ant_turn_randomness: f32,
    pub ant_update_interval: f32,
    pub ants_count: usize,
    pub ant_target_auto_pull_radius: f32,
    pub ant_steering_force_factor: f32,
    pub max_pheromone_strength: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            ph_decay_rate: PH_DECAY_RATE,
            ant_ph_strength_decay_rate: ANT_PH_STRENGTH_DECAY_RATE,
            ant_sensor_dist: ANT_SENSOR_DIST,
            ant_sensor_angle: ANT_SENSOR_ANGLE,
            ant_turn_randomness: 0.3,
            ant_update_interval: 0.1,
            ants_count: NUM_ANTS as usize,
            ant_target_auto_pull_radius: ANT_TARGET_AUTO_PULL_RADIUS,
            ant_steering_force_factor: ANT_STEERING_FORCE_FACTOR,
            max_pheromone_strength: 5000.0,
        }
    }
}

#[derive(Default, Resource)]
pub struct SimStatistics {
    pub ph_home_size: u32,
    pub ph_food_size: u32,
    pub scan_radius: f32,
    pub num_ants: usize,
    pub food_cache_size: u32,
    pub home_cache_size: u32,
}

fn settings_dialog(
    mut contexts: EguiContexts,
    mut settings: ResMut<SimSettings>,
    mut config: ResMut<SimConfig>,
    stats: Res<SimStatistics>,
    ant_query: Query<&mut Visibility, With<Ant>>,
    mut reset_sim_event: EventWriter<ResetSimEvent>,
) {
    if !settings.is_show_menu {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::Window::new("no-title")
        .title_bar(false)
        .default_pos(egui::pos2(0.0, H))
        .show(ctx, |ui| {
            egui::CollapsingHeader::new("Stats")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(format!("Food Ph: {:?}", stats.ph_food_size));
                    ui.label(format!("Home Ph: {:?}", stats.ph_home_size));
                    ui.label(format!("Food cache: {:?}", stats.food_cache_size));
                    ui.label(format!("Home cache: {:?}", stats.home_cache_size));
                    ui.label(format!("Scan radius: {:?}", stats.scan_radius.round()));
                    ui.label(format!("Num ants: {:?}", stats.num_ants));
                });
            egui::CollapsingHeader::new("Settings")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut settings.is_show_home_ph, "Home ph");
                    ui.checkbox(&mut settings.is_show_food_ph, "Food ph");
                    ui.checkbox(&mut settings.is_show_ants_path, "Paths");
                    ui.checkbox(&mut settings.is_show_sensor_radius, "Radius");
                    ui.checkbox(&mut settings.is_camera_follow, "Camera follow");
                    if ui.checkbox(&mut settings.is_show_ants, "Ants").clicked() {
                        toggle_ant_visibility(ant_query, settings.is_show_ants);
                    };
                });

            egui::CollapsingHeader::new("Parameters")
                .default_open(true)
                .show(ui, |ui| {
                    ui.add(egui::Slider::new(&mut config.ph_decay_rate, 0.01..=2.0).text("Env Ph Decay"));
                    ui.add(egui::Slider::new(&mut config.ant_ph_strength_decay_rate, 0.1..=10.0).text("Ant Ph Decay"));
                    ui.add(egui::Slider::new(&mut config.ant_sensor_dist, 5.0..=100.0).text("Sensor Dist"));
                    ui.add(egui::Slider::new(&mut config.ant_sensor_angle, 10.0..=90.0).text("Sensor Angle"));
                    ui.add(egui::Slider::new(&mut config.ant_turn_randomness, 0.0..=1.0).text("Randomness"));
                    ui.add(egui::Slider::new(&mut config.ant_update_interval, 0.01..=0.5).text("Update Interval"));
                    ui.add(egui::Slider::new(&mut config.ants_count, 0..=5000).text("Ant Count"));
                    ui.add(egui::Slider::new(&mut config.ant_target_auto_pull_radius, 10.0..=500.0).text("Attraction Radius"));
                    ui.add(egui::Slider::new(&mut config.ant_steering_force_factor, 1.0..=20.0).text("Steering Force"));
                    ui.add(egui::Slider::new(&mut config.max_pheromone_strength, 100.0..=10000.0).text("Max Pheromone"));
                });
            
            ui.add_space(10.0);
            if ui.button("Reset Simulation").clicked() {
                 reset_sim_event.send(ResetSimEvent);
            }
        });
}

fn toggle_ant_visibility(mut ant_query: Query<&mut Visibility, With<Ant>>, is_visible: bool) {
    for mut ant in ant_query.iter_mut() {
        if is_visible {
            *ant = Visibility::Visible;
        } else {
            *ant = Visibility::Hidden;
        }
    }
}



fn configure_ui(mut contexts: EguiContexts) {
    let ctx = contexts.ctx_mut();
    use bevy_egui::egui::{FontFamily, FontId, TextStyle};

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(24.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(18.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(18.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(18.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(14.0, FontFamily::Proportional)),
    ].into();
    ctx.set_style(style);
}

fn setup() {}

fn handle_camera_control(
    mut contexts: EguiContexts,
    mut cam_query: Query<&mut PanCam>,
) {
    let ctx = contexts.ctx_mut();
    let is_interacting = ctx.wants_pointer_input() || ctx.is_pointer_over_area();

    for mut pancam in cam_query.iter_mut() {
        pancam.enabled = !is_interacting;
    }
}

fn editor_ui(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut map_size: ResMut<MapSize>,
    settings: Res<SimSettings>,
    config: Res<SimConfig>,
) {
    if !settings.is_show_menu { return; }
    
    let ctx = contexts.ctx_mut();
    
    egui::Window::new("Editor Toolbar")
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -10.0))
        .title_bar(false)
        .collapsible(false)
        .show(ctx, |ui| {
             ui.horizontal(|ui| {
                 ui.selectable_value(&mut editor_state.selected_tool, EditorTool::None, "View/Select");
                 ui.selectable_value(&mut editor_state.selected_tool, EditorTool::BrushObstacle, "Draw Wall");
                 ui.selectable_value(&mut editor_state.selected_tool, EditorTool::EraserObstacle, "Eraser");
                 ui.selectable_value(&mut editor_state.selected_tool, EditorTool::PlaceFood, "Place Food");
                 ui.selectable_value(&mut editor_state.selected_tool, EditorTool::RemoveFood, "Remove Food");
                 
                 if editor_state.selected_tool == EditorTool::BrushObstacle || editor_state.selected_tool == EditorTool::EraserObstacle {
                      ui.add(egui::Slider::new(&mut editor_state.brush_size, 5.0..=100.0).text("Brush Size"));
                 }
                 
                 ui.separator();
                 ui.label("Map Size:");
                 ui.add(egui::Slider::new(&mut map_size.width, 500.0..=5000.0).text("W"));
                 ui.add(egui::Slider::new(&mut map_size.height, 500.0..=5000.0).text("H"));
                 
                 ui.separator();
                 if ui.button("Save Config").clicked() {
                     let saved = SavedConfig {
                         sim_config: (*config).clone(),
                         map_size: (*map_size).clone(),
                     };
                     if let Ok(json) = serde_json::to_string_pretty(&saved) {
                         // write to current working directory
                         if let Err(e) = std::fs::write("user_config.json", json) {
                             eprintln!("Failed to save config: {}", e);
                         } else {
                             println!("Saved config to user_config.json");
                         }
                     }
                 }
             });
        });
}

fn handle_editor_input(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut obstacle_map: ResMut<ObstacleMap>,
    map_size: Res<MapSize>,
    editor_state: Res<EditorState>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<PanCam>>,
    mouse_btn: Res<Input<MouseButton>>,
    food_query: Query<(Entity, &Transform), With<Food>>,
    mut contexts: EguiContexts,
    mut last_drag_pos: Local<Option<Vec2>>,
) {
    if editor_state.selected_tool == EditorTool::None { 
        *last_drag_pos = None;
        return; 
    }
    
    // Check if mouse is interacting with UI
    let ctx = contexts.ctx_mut();
    if ctx.is_pointer_over_area() || ctx.wants_pointer_input() {
        *last_drag_pos = None;
        return;
    }

    if camera_q.is_empty() { return; }
    
    // Reset drag if mouse not pressed
    if !mouse_btn.pressed(MouseButton::Left) {
        *last_drag_pos = None;
    }

    let (camera, camera_transform) = camera_q.single();
    if let Some(window) = windows.iter().next() {
        if let Some(cursor_pos) = window.cursor_position() {
             if let Some(ray) = camera.viewport_to_world(camera_transform, cursor_pos) {
                 let world_pos = ray.origin.truncate();
                 
                  match editor_state.selected_tool {
                      EditorTool::BrushObstacle | EditorTool::EraserObstacle => {
                          if mouse_btn.pressed(MouseButton::Left) {
                              let is_brush = editor_state.selected_tool == EditorTool::BrushObstacle;
                              
                              // Interpolation Logic
                              let start = last_drag_pos.unwrap_or(world_pos);
                              let dist = start.distance(world_pos);
                              let step = (editor_state.brush_size * 0.25).max(1.0);
                              
                              if dist > step {
                                  let steps = (dist / step).ceil() as i32;
                                  for i in 0..=steps {
                                       let t = i as f32 / steps as f32;
                                       let p = start.lerp(world_pos, t);
                                       obstacle_map.set_obstacle(p.x, p.y, map_size.width, map_size.height, is_brush, editor_state.brush_size);
                                  }
                              } else {
                                  obstacle_map.set_obstacle(world_pos.x, world_pos.y, map_size.width, map_size.height, is_brush, editor_state.brush_size);
                              }
                              
                              *last_drag_pos = Some(world_pos);
                          }
                      },
                      EditorTool::PlaceFood => {
                          if mouse_btn.just_pressed(MouseButton::Left) {
                              commands.spawn((
                                  SpriteBundle {
                                      texture: asset_server.load(SPRITE_FOOD),
                                      transform: Transform::from_xyz(world_pos.x, world_pos.y, 2.0)
                                          .with_scale(Vec3::splat(FOOD_SPRITE_SCALE)),
                                      sprite: Sprite {
                                          color: Color::rgb(1.5, 1.5, 1.5),
                                          ..default()
                                      },
                                      ..default()
                                  },
                                  Food { storage: 1000 },
                              ));
                          }
                      },
                      EditorTool::RemoveFood => {
                          if mouse_btn.pressed(MouseButton::Left) {
                              for (entity, tr) in food_query.iter() {
                                  if tr.translation.truncate().distance_squared(world_pos) < 30.0 * 30.0 {
                                      commands.entity(entity).despawn();
                                  }
                              }
                          }
                      },
                      _ => {},
                  }
             }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SavedConfig {
    sim_config: SimConfig,
    map_size: MapSize,
}

fn load_config(
    mut sim_config: ResMut<SimConfig>,
    mut map_size: ResMut<MapSize>,
) {
    // Try to load user_config.json
    if let Ok(content) = std::fs::read_to_string("user_config.json") {
       if let Ok(saved) = serde_json::from_str::<SavedConfig>(&content) {
           *sim_config = saved.sim_config;
           *map_size = saved.map_size;
           println!("Loaded user config from user_config.json");
       } else {
           println!("Failed to parse user_config.json");
       }
    } else {
        println!("No user_config.json found, using defaults.");
    }
}
