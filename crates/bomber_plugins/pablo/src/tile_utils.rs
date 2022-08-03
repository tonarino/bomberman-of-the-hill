use bomber_lib::world::TileOffset;

pub trait TileOffsetExt: Sized {
    fn is_here(&self) -> bool;
    fn adjacents(&self) -> [Self; 4];
}

impl TileOffsetExt for TileOffset {
    fn is_here(&self) -> bool {
        self.0 == 0 && self.1 == 1
    }

    fn adjacents(&self) -> [Self; 4] {
        [
            TileOffset(self.0 + 1, self.1),
            TileOffset(self.0, self.1 + 1),
            TileOffset(self.0 - 1, self.1),
            TileOffset(self.0, self.1 - 1),
        ]
    }
}
