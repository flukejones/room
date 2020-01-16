use std::f32::consts::PI;

use sdl2::{
    gfx::primitives::DrawRenderer, keyboard::Scancode, render::Canvas,
    video::Window,
};

use gamelib::{flags::LineDefFlags, map::Map};
use utils::radian_range;
use wad::{lumps::Object, lumps::Segment, Vertex, Wad};

use crate::input::Input;
use crate::{GameOptions, FP};

pub struct Game<'c> {
    input:           &'c mut Input,
    canvas:          &'c mut Canvas<Window>,
    running:         bool,
    _state_changing: bool,
    _wad:            Wad,
    map:             Map,
    player:          Object,
}

impl<'c> Game<'c> {
    /// On `Game` object creation, initialize all the game subsystems where possible
    ///
    /// Ideally full error checking will be done in by system.
    ///
    pub fn new(
        canvas: &'c mut Canvas<Window>,
        input: &'c mut Input,
        options: GameOptions,
    ) -> Game<'c> {
        let mut wad = Wad::new(options.iwad);
        wad.read_directories();
        let mut map = Map::new(options.map.unwrap_or("E1M1".to_owned()));
        map.load(&wad);

        // options.width.unwrap_or(320) as i16 / options.height.unwrap_or(200) as i16
        let map_width = map.get_extents().width as f32;
        let map_height = map.get_extents().height as f32;
        let scr_height = options.height.unwrap_or(200) as f32;
        let scr_width = options.width.unwrap_or(320) as f32;
        if map_height > map_width {
            map.set_scale(map_height / scr_height * 1.1);
        } else {
            map.set_scale(map_width / scr_width * 1.4);
        }

        let player_thing = &map.get_things()[0];
        let nodes = map.get_nodes();
        let player_subsect = map
            .find_subsector(&player_thing.pos, (nodes.len() - 1) as u16)
            .unwrap();

        let player = Object::new(
            player_thing.pos.clone(),
            player_subsect.sector.floor_height as f32,
            player_thing.angle * PI / 180.0,
            player_subsect.sector.clone(),
        );

        dbg!(&player);

        Game {
            input,
            canvas,
            running: true,
            _state_changing: false,
            _wad: wad,
            map,
            player,
        }
    }

    /// Called by the main loop
    pub fn update(&mut self, time: FP) {
        let rot_amnt = 0.11 * time;
        let mv_amnt = 25.0 * time;
        if self.input.get_key(Scancode::Left) {
            self.player.rotation =
                radian_range(self.player.rotation + rot_amnt);
        }

        if self.input.get_key(Scancode::Right) {
            self.player.rotation =
                radian_range(self.player.rotation - rot_amnt);
        }

        if self.input.get_key(Scancode::Up) {
            let heading = self.player.rotation.sin_cos();
            self.player
                .xy
                .set_x(self.player.xy.x() + heading.1 * mv_amnt);
            self.player
                .xy
                .set_y(self.player.xy.y() + heading.0 * mv_amnt);
        }

        if self.input.get_key(Scancode::Down) {
            let heading = self.player.rotation.sin_cos();
            self.player
                .xy
                .set_x(self.player.xy.x() - heading.1 * mv_amnt);
            self.player
                .xy
                .set_y(self.player.xy.y() - heading.0 * mv_amnt);
        }
    }

    /// `handle_events` updates the current events and inputs plus changes `states`
    ///
    /// In an C++ engine, using a button to switch states would probably be
    /// handled in the state itself. We can't do that with rust as it requires
    /// passing a mutable reference to the state machine to the state; essentially
    /// this is the same as an object in an Vec<Type> trying to modify its container.
    ///
    /// So because of the above reasons, `states::States` does not allow a game state
    /// to handle state changes or fetching
    ///
    pub fn handle_events(&mut self) {
        self.input.update();
        self.running = !self.input.get_quit();

        if self.input.get_key(Scancode::Escape) {
            self.running = false;
        }
    }

    /// `render` calls the `states.render()` method with a time-step for state renders
    ///
    /// The main loop, in this case, the `'running : loop` in lib.rs should calculate
    /// a time-step to pass down through the render functions for use in the game states,
    /// from which the game states (or menu) will use to render objects at the correct
    /// point in time.
    ///
    pub fn render(&mut self, dt: FP) {
        // The state machine will handle which state renders to the surface
        //self.states.render(dt, &mut self.canvas);
        let nodes = self.map.get_nodes();
        let player_subsect = self
            .map
            .find_subsector(&self.player.xy, (nodes.len() - 1) as u16)
            .unwrap();
        self.player.z = player_subsect.sector.floor_height as f32;
        self.player.sector = player_subsect.sector.clone();

        self.draw_automap();
        self.canvas.present();
    }

    /// Called by the main loop
    pub fn running(&self) -> bool {
        self.running
    }

    fn vertex_to_screen(&self, v: &Vertex) -> (i16, i16) {
        let scale = self.map.get_extents().automap_scale;
        let scr_height = self.canvas.viewport().height() as f32;
        let scr_width = self.canvas.viewport().width() as f32;

        let x_pad = (scr_width * scale - self.map.get_extents().width) / 2.0;
        let y_pad = (scr_height * scale - self.map.get_extents().height) / 2.0;

        let x_shift = -self.map.get_extents().min_vertex.x() + x_pad;
        let y_shift = -self.map.get_extents().min_vertex.y() + y_pad;
        (
            ((v.x() + x_shift) / scale) as i16,
            (scr_height - (v.y() + y_shift) / scale) as i16,
        )
    }

    /// Testing function
    pub fn draw_automap(&mut self) {
        let yel = sdl2::pixels::Color::RGBA(255, 255, 50, 255);
        // clear background to black
        self.canvas
            .set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 255));
        self.canvas.clear();

        for linedef in self.map.get_linedefs() {
            let start = self.vertex_to_screen(&linedef.start_vertex);
            let end = self.vertex_to_screen(&linedef.end_vertex);
            let draw_colour =
                if linedef.flags & LineDefFlags::TwoSided as u16 == 0 {
                    sdl2::pixels::Color::RGBA(160, 70, 70, 255)
                } else if linedef.flags & LineDefFlags::Secret as u16
                    == LineDefFlags::Secret as u16
                {
                    sdl2::pixels::Color::RGBA(100, 255, 255, 255)
                } else if linedef.line_type != 0 {
                    yel
                } else {
                    sdl2::pixels::Color::RGBA(148, 148, 148, 255)
                };
            self.canvas
                .thick_line(start.0, start.1, end.0, end.1, 2, draw_colour)
                .unwrap();
        }

        self.map.draw_bsp(
            &self.player,
            self.map.start_node(),
            &mut self.canvas,
        );

        let player = self.vertex_to_screen(&self.player.xy);

        let (py, px) = self.player.rotation.sin_cos();
        let (lpy, lpx) = (self.player.rotation + PI / 4.0).sin_cos();
        let (rpy, rpx) = (self.player.rotation - PI / 4.0).sin_cos();
        self.canvas
            .thick_line(
                player.0,
                player.1,
                player.0 + (px * 25.0) as i16,
                player.1 - (py * 25.0) as i16,
                2,
                yel,
            )
            .unwrap();
        self.canvas
            .thick_line(
                player.0,
                player.1,
                player.0 + (lpx * 500.0) as i16,
                player.1 - (lpy * 500.0) as i16,
                2,
                yel,
            )
            .unwrap();
        self.canvas
            .thick_line(
                player.0,
                player.1,
                player.0 + (rpx * 500.0) as i16,
                player.1 - (rpy * 500.0) as i16,
                2,
                yel,
            )
            .unwrap();
    }
}
