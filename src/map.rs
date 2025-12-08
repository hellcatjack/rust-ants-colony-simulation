use bevy::prelude::*;
use crate::configs::{W, H, PH_UNIT_GRID_SIZE};
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize, Clone, Copy)]
pub struct MapSize {
    pub width: f32,
    pub height: f32,
}

impl Default for MapSize {
    fn default() -> Self {
        Self {
            width: W,
            height: H,
        }
    }
}

#[derive(Resource)]
pub struct ObstacleMap {
    pub grid: Vec<bool>, // true = obstacle
    pub width: usize,
    pub height: usize,
    pub texture_handle: Handle<Image>,
}

impl Default for ObstacleMap {
    fn default() -> Self {
        Self {
            grid: vec![], // Initialized in setup
            width: 0,
            height: 0,
            texture_handle: Handle::default(),
        }
    }
}

impl ObstacleMap {
    pub fn new(w: f32, h: f32) -> Self {
        let cols = (w as usize / PH_UNIT_GRID_SIZE) + 1;
        let rows = (h as usize / PH_UNIT_GRID_SIZE) + 1;
        Self {
            grid: vec![false; cols * rows],
            width: cols,
            height: rows,
            texture_handle: Handle::default(),
        }
    }

    pub fn is_obstacle_in_radius(&self, x: f32, y: f32, radius: f32, map_w: f32, map_h: f32) -> bool {
        let grid_radius = (radius / PH_UNIT_GRID_SIZE as f32).ceil() as isize;
        let center_grid_x = ((x + map_w / 2.0) / PH_UNIT_GRID_SIZE as f32) as isize;
        let center_grid_y = ((y + map_h / 2.0) / PH_UNIT_GRID_SIZE as f32) as isize;

        // Optimization: check center first
        if self.is_obstacle_at_index(center_grid_x, center_grid_y) { return true; }
        
        // Check bounding box in grid
        for dy in -grid_radius..=grid_radius {
            for dx in -grid_radius..=grid_radius {
                if dx == 0 && dy == 0 { continue; }
                
                // Simple distance check to preserve circle shape roughly
                // Or just box check is fine for "at least one touch"
                if (dx*dx + dy*dy) as f32 > (grid_radius as f32 * grid_radius as f32) + 1.0 { continue; }
                
                let gx = center_grid_x + dx;
                let gy = center_grid_y + dy;
                
                if self.is_obstacle_at_index(gx, gy) {
                    return true;
                }
            }
        }
        false
    }
    
    fn is_obstacle_at_index(&self, gx: isize, gy: isize) -> bool {
         if gx < 0 || gx >= self.width as isize || gy < 0 || gy >= self.height as isize {
             return true; // Treat OOB as obstacle
         }
         self.grid[gy as usize * self.width + gx as usize]
    }
    
    pub fn is_obstacle(&self, x: f32, y: f32, map_w: f32, map_h: f32) -> bool {
         let grid_x = ((x + map_w / 2.0) / PH_UNIT_GRID_SIZE as f32) as isize;
         let grid_y = ((y + map_h / 2.0) / PH_UNIT_GRID_SIZE as f32) as isize;
         self.is_obstacle_at_index(grid_x, grid_y)
    }

    pub fn set_obstacle(&mut self, x: f32, y: f32, map_w: f32, map_h: f32, is_obstacle: bool, brush_size: f32) {
        let center_grid_x = ((x + map_w / 2.0) / PH_UNIT_GRID_SIZE as f32) as isize;
        let center_grid_y = ((y + map_h / 2.0) / PH_UNIT_GRID_SIZE as f32) as isize;
        let radius = (brush_size / PH_UNIT_GRID_SIZE as f32).ceil() as isize;

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx*dx + dy*dy > radius*radius { continue; }
                
                let gx = center_grid_x + dx;
                let gy = center_grid_y + dy;

                if gx >= 0 && gx < self.width as isize && gy >= 0 && gy < self.height as isize {
                    self.grid[gy as usize * self.width + gx as usize] = is_obstacle;
                }
            }
        }
    }
    
    pub fn clear(&mut self) {
        self.grid.fill(false);
    }
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MapSize::default())
           .insert_resource(ObstacleMap::new(W, H))
           .add_systems(Startup, setup_obstacle_texture)
           .add_systems(Update, (resize_obstacle_map, update_obstacle_texture));
    }
}

// ... setup_obstacle_texture ...

fn setup_obstacle_texture(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut obstacle_map: ResMut<ObstacleMap>,
    _map_size: Res<MapSize>,
) {
    let w = obstacle_map.width;
    let h = obstacle_map.height;
    
    // Create a transparent image
    let image = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[0, 0, 0, 0],
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
    );
    
    let handle = images.add(image);
    obstacle_map.texture_handle = handle.clone();

    commands.spawn(SpriteBundle {
        texture: handle,
        transform: Transform::from_xyz(0.0, 0.0, 1.0) // Z-index 1.0 (below ants, above bg)
            .with_scale(Vec3::splat(PH_UNIT_GRID_SIZE as f32)),
        ..default()
    });
}


fn resize_obstacle_map(
    mut obstacle_map: ResMut<ObstacleMap>,
    map_size: Res<MapSize>,
    mut images: ResMut<Assets<Image>>,
) {
    if map_size.is_changed() {
        let new_w = (map_size.width as usize / PH_UNIT_GRID_SIZE) + 1;
        let new_h = (map_size.height as usize / PH_UNIT_GRID_SIZE) + 1;
        
        if new_w != obstacle_map.width || new_h != obstacle_map.height {
            // Resize grid, preserving old data if possible? 
            // For now, simpler to clear or create new. Let's just create new to match the requested size perfectly.
            // Or better, creating a new struct instance but keeping texture handle?
            obstacle_map.width = new_w;
            obstacle_map.height = new_h;
            obstacle_map.grid = vec![false; new_w * new_h];
            
            // Allow OOB logic to work correctly now with new dimensions.

            // Also resize texture
            if let Some(image) = images.get_mut(&obstacle_map.texture_handle) {
                 image.resize(bevy::render::render_resource::Extent3d {
                    width: new_w as u32,
                    height: new_h as u32,
                    depth_or_array_layers: 1,
                });
                // Initialize with transparent
                image.data = vec![0; new_w * new_h * 4];
            }
        }
    }
}

fn update_obstacle_texture(
    obstacle_map: Res<ObstacleMap>,
    mut images: ResMut<Assets<Image>>,
) {
    if obstacle_map.is_changed() {
        if let Some(image) = images.get_mut(&obstacle_map.texture_handle) {
            // Check if sizes match to avoid panic during resize
            if image.size().x as usize != obstacle_map.width || image.size().y as usize != obstacle_map.height {
                return;
            }

            for y in 0..obstacle_map.height {
                // Flip Y for display because World Y-up corresponds to Image Y-down usually in these mappings
                // Our grid: 0 is bottom (-H/2). Image: 0 is top.
                let img_y = obstacle_map.height.saturating_sub(1) - y;
                
                for x in 0..obstacle_map.width {
                    let idx = y * obstacle_map.width + x;
                    let is_obs = obstacle_map.grid[idx];
                    
                    let pixel_idx = (img_y * obstacle_map.width + x) * 4;
                    
                    if pixel_idx + 3 < image.data.len() {
                        if is_obs {
                            image.data[pixel_idx] = 100;   // R
                            image.data[pixel_idx + 1] = 100; // G
                            image.data[pixel_idx + 2] = 100; // B
                            image.data[pixel_idx + 3] = 255; // Alpha
                        } else {
                            image.data[pixel_idx + 3] = 0;   // Transparent
                        }
                    }
                }
            }
        }
    }
}
