pub const SCALE_PX: f32 = 0.5;

pub const TILE_WIDTH_PX: f32 = 64.0 * SCALE_PX;
pub const TILE_HEIGHT_PX: f32 = 64.0 * SCALE_PX;

pub const GAME_MAP_Z: f32 = 0.0;
pub const GAME_OBJECT_Z: f32 = GAME_MAP_Z + 1.0;
pub const PLAYER_Z: f32 = GAME_OBJECT_Z + 1.0;
pub const FLAME_Z: f32 = PLAYER_Z + 1.0;

pub const PLAYER_WIDTH_PX: f32 = 64.0 * SCALE_PX;
pub const PLAYER_HEIGHT_PX: f32 = 128.0 * SCALE_PX;
pub const PLAYER_VERTICAL_OFFSET_PX: f32 = (PLAYER_HEIGHT_PX - TILE_HEIGHT_PX) / 2.0;
