use bevy::{
    asset::Handle,
    ecs::{component::Component, entity::Entity, system::Commands},
    math::{UVec2, Vec2},
    render::{color::Color, texture::Image},
};

#[derive(Component, Clone, Copy)]
pub struct TilemapSlotSize(pub Vec2);

#[derive(Component, Clone)]
pub struct TilemapTexture {
    pub image: Handle<Image>,
    pub image_size: UVec2,
    pub tile_size: UVec2,
}

#[derive(Component)]
pub struct TilemapStorage {
    storage: Vec<Option<Entity>>,
    size: UVec2,
}

impl TilemapStorage {
    pub fn new(size: UVec2) -> Self {
        Self {
            storage: vec![None; (size.x * size.y) as usize],
            size,
        }
    }

    #[inline]
    pub fn set(&mut self, commands: &mut Commands, index: UVec2, tile: Tile) {
        self.storage[(index.y * self.size.x + index.x) as usize] =
            Some(commands.spawn((tile, TileIndex(index))).id());
    }
}

#[derive(Component, Default, Clone)]
pub struct TilemapAnimation {
    // fps frame1 frame2 frame3 fps frame1 frame2 ...
    buffer: Vec<u32>,
}

impl TilemapAnimation {
    pub fn add_animation(&mut self, anim: Vec<u32>, fps: u32) -> TileAnimation {
        self.buffer.push(fps);
        let start = self.buffer.len() as u32 - 1;
        let length = anim.len() as u32 - 1;
        self.buffer.extend(anim);
        TileAnimation { start, length }
    }
}

#[derive(Clone, Copy)]
pub enum TileTexture {
    Static(u32),
    Animated(TileAnimation),
}

#[derive(Clone, Copy)]
pub struct TileAnimation {
    start: u32,
    length: u32,
}

#[derive(Component)]
pub struct Tile {
    pub tilemap: Entity,
    pub texture: TileTexture,
    pub tint: Color,
}

#[derive(Component)]
pub struct TileIndex(pub UVec2);
