use hero_lib::world::Tile;

pub struct Labyrinth {
    tiles: Vec<Vec<Tile>>,
}

pub struct Location(pub usize, pub usize);

impl<T: AsRef<str>> From<T> for Labyrinth {
    fn from(text: T) -> Self {
        let lines: Vec<&str> = text.as_ref().lines().rev().collect();
        println!("{}", lines[0]);

        // Very panicky (this should be a TryFrom) but good for a quick test
        assert!(lines.windows(2).all(|w| w[0].len() == w[1].len()));
        assert!(lines.len() > 0 && lines[0].len() > 0);
        let convert_line = |l: &str| -> Vec<Tile> {
            l.chars().map(Into::into).collect()
        };

        Self { tiles: lines.into_iter().map(convert_line).collect() }
    }
}

impl Labyrinth {
    pub fn size(&self) -> (usize, usize) {
        (self.tiles.len(), self.tiles[0].len())
    }

    pub fn tile(&self, location: Location) -> Option<Tile> {
        self.tiles.get(location.1).and_then(|v| v.get(location.0)).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_labyrinths() {
        let labyrinth_text =
            "####.###\n\
             #......#\n\
             #.####.#\n\
             #..##..#\n\
             #X.##..#\n\
             #......#\n\
             ####.###";
        let labyrinth = Labyrinth::from(labyrinth_text);
        assert_eq!(labyrinth.size(), (7, 8));
        assert_eq!(labyrinth.tile(Location(0, 0)).unwrap(), Tile::Wall);
        assert_eq!(labyrinth.tile(Location(4, 0)).unwrap(), Tile::EmptyFloor);
        assert_eq!(labyrinth.tile(Location(1, 1)).unwrap(), Tile::EmptyFloor);
        assert_eq!(labyrinth.tile(Location(1, 2)).unwrap(), Tile::Lava);
        assert_eq!(labyrinth.tile(Location(8, 8)), None);
    }
}
