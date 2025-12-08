use crate::{ant::Ant, *};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

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
}

#[derive(Event)]
pub struct ResetSimEvent;

#[derive(Resource)]
pub struct SimConfig {
    pub ph_decay_rate: f32,
    pub ant_ph_strength_decay_rate: f32,
    pub ant_sensor_dist: f32,
    pub ant_sensor_angle: f32,
    pub ant_turn_randomness: f32,
    pub ant_update_interval: f32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            ph_decay_rate: PH_DECAY_RATE,
            ant_ph_strength_decay_rate: ANT_PH_STRENGTH_DECAY_RATE,
            ant_sensor_dist: ANT_SENSOR_DIST,
            ant_sensor_angle: ANT_SENSOR_ANGLE,
            ant_turn_randomness: 0.3, // Approximation of hardcoded value
            ant_update_interval: 0.1, // Default to slightly lower frequency than original 0.05
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

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(SimSettings::default())
            .insert_resource(SimStatistics::default())
            .insert_resource(SimConfig::default())
            .add_event::<ResetSimEvent>()
            .add_systems(Update, settings_dialog)
            .add_systems(Update, settings_toggle)
            .add_plugins(EguiPlugin)
            .add_systems(Startup, (setup, configure_ui));
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

fn settings_toggle(
    mut settings: ResMut<SimSettings>,
    ant_query: Query<&mut Visibility, With<Ant>>,
    keys: Res<Input<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        settings.is_show_menu = !settings.is_show_menu;
    }
    if keys.just_pressed(KeyCode::H) {
        settings.is_show_home_ph = !settings.is_show_home_ph;
    }
    if keys.just_pressed(KeyCode::F) {
        settings.is_show_food_ph = !settings.is_show_food_ph;
    }
    if keys.just_pressed(KeyCode::P) {
        settings.is_show_ants_path = !settings.is_show_ants_path;
    }
    if keys.just_pressed(KeyCode::A) {
        settings.is_show_ants = !settings.is_show_ants;
        toggle_ant_visibility(ant_query, settings.is_show_ants);
    }
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

impl Default for SimSettings {
    fn default() -> Self {
        Self {
            is_show_home_ph: true,
            is_show_food_ph: true,
            is_show_ants: true,
            is_camera_follow: false,
            is_show_menu: false,
            is_show_ants_path: false,
            is_show_sensor_radius: false,
        }
    }
}
