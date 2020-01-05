use crate::map::{DPtr, LineDef, Map, Sector, Segment, SideDef, SubSector, Thing, Vertex};
use std::fs::File;
use std::intrinsics::transmute;
use std::io::prelude::*;
use std::ops::Sub;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::{fmt, str};

/// Used as an index to find a specific lump, typically combined
/// with an offset for example: find the index for lump named "E1M1"
/// in `self.wad_dirs` then combine this index with a `LumpIndex`
/// variant to get a specific lump.
#[allow(dead_code)]
enum LumpIndex {
    /// Position and angle for all monster, powerup and spawn location
    Things = 1,
    /// An array of lines referencing two vertices (Two vertexes are connected
    /// by one `LineDef`). This is a direct
    /// translation of the lines used in DoomED. Also points to one or two
    /// `SideDef` depending on if this line is a wall or a portal
    LineDefs,
    /// Defines upper, lower, and middle textures. Also defines texture
    /// horizontal and vertical offsets. This is information for a `LineDef`
    SideDefs,
    /// An array of signed short X, Y pairs (`Vertex`). All coordinates in this map
    /// block are indexes into this array
    Vertexes,
    /// Portions of lines cut due to Binary Space Partitioning (see page
    /// 202 in Fabien Sanglard's Game Engine Black Book - DOOM).
    /// Each `SubSectors`'s geometry is defined by the `Segs` which it contains
    Segs,
    /// Set of segments of a `LineDef` representing a convex subspace
    SubSectors,
    /// BSP with segs, nodes and sub-sector leaves
    Nodes,
    /// Area surrounded by lines, with set ceiling and floor textures/heights
    /// with light level
    Sectors,
    /// Sector-to-sector visibility matrix to speed-up line of sight
    /// calculations
    Reject,
    /// 128x128 grid partition of the map LINEDEFS to accelerate collision
    /// detection
    Blockmap,
    Count,
}

impl LumpIndex {
    fn to_string(&self) -> String {
        match self {
            LumpIndex::Things => "THINGS".to_string(),
            LumpIndex::LineDefs => "LINEDEFS".to_string(),
            LumpIndex::SideDefs => "SIDEDEFS".to_string(),
            LumpIndex::Vertexes => "VERTEXES".to_string(),
            LumpIndex::Segs => "SEGS".to_string(),
            LumpIndex::SubSectors => "SSECTORS".to_string(),
            LumpIndex::Nodes => "NODES".to_string(),
            LumpIndex::Sectors => "SECTORS".to_string(),
            LumpIndex::Reject => "REJECT".to_string(),
            LumpIndex::Blockmap => "BLOCKMAP".to_string(),
            LumpIndex::Count => "COUNT".to_string(),
        }
    }
}

/// Header which tells us the WAD type and where the data is
struct WadHeader {
    /// Will be either `IWAD` for game, or `PWAD` for patch
    wad_type: [u8; 4],
    /// The count of "lumps" of data
    dir_count: u32,
    /// Offset in bytes that the lump data starts at
    dir_offset: u32,
}

impl fmt::Debug for WadHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\nWadHeader {{\n  wad_type: {},\n  dir_count: {},\n  dir_offset: {},\n}}",
            str::from_utf8(&self.wad_type).unwrap(),
            self.dir_count,
            self.dir_offset
        )
    }
}

/// Contains the details for a lump of data: where it starts, the size of it, and the name
struct WadDirectory {
    /// The offset in bytes where the lump data starts
    lump_offset: u32,
    /// The size in bytes of the lump referenced
    lump_size: u32,
    /// Name for the lump data
    lump_name: String,
}
impl fmt::Debug for WadDirectory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\nWadDirectory {{\n  lump_name: {},\n  lump_size: {},\n  lump_offset: {},\n}}",
            &self.lump_name, self.lump_size, self.lump_offset
        )
    }
}

/// "Where's All (the) Data": contains the WAD in memory, plus an array of directories
/// telling us where each data lump starts
pub struct Wad {
    wad_file_path: PathBuf,
    /// The WAD as an array of bytes read in to memory
    wad_data: Vec<u8>,
    /// Tells us where each lump of data is
    wad_dirs: Vec<WadDirectory>,
}

impl fmt::Debug for Wad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\nWadLoader {{\n  wad_file_path: {:?},\n wad_dirs: {:?},\n}}",
            self.wad_file_path, self.wad_dirs
        )
    }
}

impl Wad {
    pub fn new<A>(file_path: A) -> Wad
    where
        A: Into<PathBuf>,
    {
        let mut wad = Wad {
            wad_file_path: file_path.into(),
            wad_data: Vec::new(),
            wad_dirs: Vec::new(),
        };

        let mut file = File::open(&wad.wad_file_path)
            .expect(&format!("Could not open {:?}", &wad.wad_file_path));

        let file_len = file.metadata().unwrap().len();
        wad.wad_data.reserve_exact(file_len as usize);
        let wad_len = file
            .read_to_end(&mut wad.wad_data)
            .expect(&format!("Could not read {:?}", &wad.wad_file_path));

        if wad_len != file_len as usize {
            panic!("Did not read complete WAD")
        }

        wad
    }

    fn read_2_bytes(&self, offset: usize) -> u16 {
        (self.wad_data[offset + 1] as u16) << 8 | (self.wad_data[offset] as u16)
    }

    fn read_4_bytes(&self, offset: usize) -> u32 {
        (self.wad_data[offset + 3] as u32) << 24
            | (self.wad_data[offset + 2] as u32) << 16
            | (self.wad_data[offset + 1] as u32) << 8
            | (self.wad_data[offset] as u32)
    }

    fn read_header(&self, offset: usize) -> WadHeader {
        let mut t = [0u8; 4];
        t[0] = self.wad_data[offset];
        t[1] = self.wad_data[offset + 1];
        t[2] = self.wad_data[offset + 2];
        t[3] = self.wad_data[offset + 3];

        WadHeader {
            wad_type: t,
            dir_count: self.read_4_bytes(offset + 4),
            dir_offset: self.read_4_bytes(offset + 8),
        }
    }

    fn read_dir_data(&self, offset: usize) -> WadDirectory {
        let mut n = [0u8; 8]; // length is 8 slots total
        for i in 0..8 {
            n[i] = self.wad_data[offset + 8 + i]
        }

        WadDirectory {
            lump_offset: self.read_4_bytes(offset),
            lump_size: self.read_4_bytes(offset + 4),
            lump_name: str::from_utf8(&n)
                .expect("Invalid lump name")
                .trim_end_matches("\u{0}") // better to address this early to avoid many casts later
                .to_owned(),
        }
    }

    pub fn read_directories(&mut self) {
        let header = self.read_header(0);
        self.wad_dirs.reserve_exact(header.dir_count as usize);

        for i in 0..(header.dir_count) {
            let dir = self.read_dir_data((header.dir_offset + i * 16) as usize);
            self.wad_dirs.push(dir);
        }
    }

    pub fn find_lump_index(&self, name: &str) -> usize {
        for (i, dir) in self.wad_dirs.iter().enumerate() {
            if &dir.lump_name == name {
                return i;
            }
        }
        panic!("Index not found for lump name: {}", name);
    }

    fn read_lump_to_vec<F, T>(
        &self,
        mut index: usize,
        lump_type: LumpIndex,
        data_size: u32,
        func: F,
    ) -> Vec<T>
    where
        F: Fn(usize) -> T,
    {
        let name: String = lump_type.to_string();
        index += lump_type as usize;

        if self.wad_dirs[index].lump_name != name {
            panic!(
                "Invalid {} lump index: {}, found {}",
                name, index, self.wad_dirs[index].lump_name
            )
        }

        let data_count = self.wad_dirs[index].lump_size / data_size;

        let mut v: Vec<T> = Vec::new();
        for i in 0..data_count {
            let offset = (self.wad_dirs[index].lump_offset + i * data_size) as usize;
            v.push(func(offset));
        }
        v
    }

    pub fn load_map<'m>(&self, mut map: &'m mut Map) {
        let index = self.find_lump_index("E1M1");
        // THINGS
        map.set_things(
            self.read_lump_to_vec(index, LumpIndex::Things, 10, |offset| {
                Thing::new(
                    self.read_2_bytes(offset) as i16,
                    self.read_2_bytes(offset + 2) as i16,
                    self.read_2_bytes(offset + 4),
                    self.read_2_bytes(offset + 6),
                    self.read_2_bytes(offset + 8),
                )
            }),
        );
        // Vertexes
        map.set_vertexes(
            self.read_lump_to_vec(index, LumpIndex::Vertexes, 4, |offset| {
                Vertex::new(
                    self.read_2_bytes(offset) as i16,
                    self.read_2_bytes(offset + 2) as i16,
                )
            }),
        );
        // Sidedefs
        map.set_sidedefs(
            self.read_lump_to_vec(index, LumpIndex::SideDefs, 30, |offset| {
                SideDef::new(
                    self.read_2_bytes(offset) as i16,
                    self.read_2_bytes(offset + 2) as i16,
                    &self.wad_data[offset + 4..offset + 12],
                    &self.wad_data[offset + 12..offset + 20],
                    &self.wad_data[offset + 20..offset + 28],
                    self.read_2_bytes(offset + 28),
                )
            }),
        );
        //LineDefs
        map.set_linedefs(
            self.read_lump_to_vec(index, LumpIndex::LineDefs, 14, |offset| {
                let start_vertex = &map.get_vertexes()[self.read_2_bytes(offset) as usize];
                let end_vertex = &map.get_vertexes()[self.read_2_bytes(offset + 2) as usize];
                let front_sidedef = &map.get_sidedefs()[self.read_2_bytes(offset + 10) as usize];
                let back_sidedef = {
                    let index = self.read_2_bytes(offset + 12) as usize;
                    if index < 65535 {
                        Some(DPtr::new(&map.get_sidedefs()[index]))
                    } else {
                        None
                    }
                };
                unsafe {
                    LineDef::new(
                        DPtr::new(start_vertex),
                        DPtr::new(end_vertex),
                        self.read_2_bytes(offset + 4),
                        self.read_2_bytes(offset + 6),
                        self.read_2_bytes(offset + 8),
                        DPtr::new(front_sidedef),
                        back_sidedef,
                    )
                }
            }),
        );
        // Sectors
        map.set_sectors(
            self.read_lump_to_vec(index, LumpIndex::Sectors, 26, |offset| {
                Sector::new(
                    self.read_2_bytes(offset) as i16,
                    self.read_2_bytes(offset + 2) as i16,
                    &self.wad_data[offset + 4..offset + 12],
                    &self.wad_data[offset + 12..offset + 20],
                    self.read_2_bytes(offset + 20),
                    self.read_2_bytes(offset + 22),
                    self.read_2_bytes(offset + 24),
                )
            }),
        );
        // Sector, Sidedef, Linedef, Seg all need to be preprocessed before
        // storing in map struct
        //
        // SEGS
        map.set_segments(self.read_lump_to_vec(index, LumpIndex::Segs, 12, |offset| {
            let start_vertex = &map.get_vertexes()[self.read_2_bytes(offset) as usize];
            let end_vertex = &map.get_vertexes()[self.read_2_bytes(offset + 2) as usize];
            unsafe {
                Segment::new(
                    DPtr::new(start_vertex),
                    DPtr::new(end_vertex),
                    self.read_2_bytes(offset + 4),
                    self.read_2_bytes(offset + 6),
                    self.read_2_bytes(offset + 8),
                    self.read_2_bytes(offset + 10),
                )
            }
        }));
        // SSECTORS
        map.set_subsectors(self.read_lump_to_vec(
            index,
            LumpIndex::SubSectors,
            4,
            |offset| unsafe {
                let seg = &map.get_segments()[self.read_2_bytes(offset + 2) as usize];
                SubSector::new(self.read_2_bytes(offset), DPtr::new(seg))
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use crate::map;
    use crate::wad::Wad;

    #[test]
    fn load_wad() {
        let mut wad = Wad::new("../doom1.wad");
        assert_eq!(wad.wad_data.len(), 4225460);
    }

    #[test]
    fn read_two_bytes() {
        let wad = Wad::new("../doom1.wad");
        let x1 = wad.read_2_bytes(0);
        dbg!(&x1);
        let x2 = wad.read_2_bytes(2);
        dbg!(&x2);
    }

    #[test]
    fn read_four_bytes() {
        let wad = Wad::new("../doom1.wad");
        let x = wad.read_4_bytes(0);
        dbg!(&x);

        let y = (wad.read_2_bytes(2) as u32) << 16 | (wad.read_2_bytes(0) as u32);
        dbg!(&y);

        assert_eq!(x, y);
    }

    #[test]
    fn read_header() {
        let wad = Wad::new("../doom1.wad");

        let header = wad.read_header(0);
        dbg!(&header);
    }

    #[test]
    fn read_single_dir() {
        let wad = Wad::new("../doom1.wad");

        let header = wad.read_header(0);
        let dir = wad.read_dir_data((header.dir_offset) as usize);
        dbg!(&dir);
    }

    #[test]
    fn read_all_dirs() {
        let mut wad = Wad::new("../doom1.wad");
        wad.read_directories();

        for i in 6..18 {
            dbg!(&wad.wad_dirs[i]);
        }

        let header = wad.read_header(0);
        assert_eq!(wad.wad_dirs.len(), header.dir_count as usize);
    }

    #[test]
    fn find_e1m1() {
        let mut wad = Wad::new("../doom1.wad");
        wad.read_directories();

        assert_eq!(wad.wad_dirs[6].lump_name, "E1M1");

        let i = wad.find_lump_index("E1M1");
        assert_eq!(wad.wad_dirs[i].lump_name, "E1M1");
    }
}
