// TODO: Why power of two?
pub enum LineDefFlags {
    Blocking = 0,
    BlockMonsters = 1,
    TwoSided = 2,
    DontPegTop = 4,
    DontPegBottom = 8,
    Secret = 16,
    SoundBlock = 32,
    DontDraw = 64,
    Draw = 128,
}

#[derive(Debug)]
pub struct Vertex {
    pub x_pos: i16,
    pub y_pos: i16,
}

impl Vertex {
    pub fn new(x: i16, y: i16) -> Vertex {
        Vertex { x_pos: x, y_pos: y }
    }
}

#[derive(Debug)]
pub struct LineDef {
    pub start_vertex: i16,
    pub end_vertex: i16,
    pub flags: u16, //TODO: enum?
    pub line_type: u16,
    pub sector_tag: u16,
    pub front_sidedef: u16, //0xFFFF means there is no sidedef
    pub back_sidedef: u16,  //0xFFFF means there is no sidedef
}

impl LineDef {
    pub fn new(
        start_vertex: i16,
        end_vertex: i16,
        flags: u16,
        line_type: u16,
        sector_tag: u16,
        front_sidedef: u16,
        back_sidedef: u16,
    ) -> LineDef {
        LineDef {
            start_vertex,
            end_vertex,
            flags,
            line_type,
            sector_tag,
            front_sidedef,
            back_sidedef,
        }
    }
}

pub struct Map {
    pub name: String,
    pub vertexes: Vec<Vertex>,
    pub linedefs: Vec<LineDef>,
}

impl Map {
    pub fn new(name: String) -> Map {
        Map {
            name,
            vertexes: Vec::new(),
            linedefs: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::map;
    use crate::wad::Wad;

    #[test]
    fn load_e1m1_vertexes() {
        let mut wad = Wad::new("../doom1.wad");
        wad.read_directories();

        let mut map = map::Map::new("E1M1".to_owned());
        let index = wad.find_lump_index(&map.name);
        wad.read_map_vertexes(index, &mut map);

        assert_eq!(map.vertexes[0].x_pos, 1088);
        assert_eq!(map.vertexes[0].y_pos, -3680);
    }

    #[test]
    fn load_e1m1_linedefs() {
        let mut wad = Wad::new("../doom1.wad");
        wad.read_directories();

        let mut map = map::Map::new("E1M1".to_owned());
        let index = wad.find_lump_index(&map.name);
        wad.read_map_linedefs(index, &mut map);

        let linedefs = map.linedefs;
        assert_eq!(linedefs[0].start_vertex, 0);
        assert_eq!(linedefs[0].end_vertex, 1);
        assert_eq!(linedefs[2].start_vertex, 3);
        assert_eq!(linedefs[2].end_vertex, 0);
        assert_eq!(linedefs[2].front_sidedef, 2);
        assert_eq!(linedefs[2].back_sidedef, 65535);
    }
}
