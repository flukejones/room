use std::{error::Error, fmt, str::FromStr};

use gumdrop::Options;
use sdl2::{
    keyboard::Scancode, pixels::PixelFormatEnum, render::Canvas,
    surface::Surface, video::Window,
};

use crate::{input::Input, timestep::TimeStep, Game};

#[derive(Debug)]
pub enum DoomArgError {
    InvalidSkill(String),
}

impl Error for DoomArgError {}

impl fmt::Display for DoomArgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DoomArgError::InvalidSkill(m) => write!(f, "{}", m),
        }
    }
}

#[derive(Debug)]
pub enum Skill {
    NoItems = -1, // the "-skill 0" hack
    Baby    = 0,
    Easy,
    Medium,
    Hard,
    Nightmare,
}

impl Default for Skill {
    fn default() -> Self { Skill::Medium }
}

impl FromStr for Skill {
    type Err = DoomArgError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(Skill::Baby),
            "1" => Ok(Skill::Easy),
            "2" => Ok(Skill::Medium),
            "3" => Ok(Skill::Hard),
            "4" => Ok(Skill::Nightmare),
            _ => Err(DoomArgError::InvalidSkill("Invalid arg".to_owned())),
        }
    }
}

#[derive(Debug, Options)]
pub struct GameOptions {
    #[options(help = "path to game WAD", default = "./doom1.wad")]
    pub iwad:       String,
    #[options(help = "path to patch WAD")]
    pub pwad:       Option<String>,
    #[options(help = "resolution width in pixels", default = "640")]
    pub width:      u32,
    #[options(help = "resolution height in pixels", default = "480")]
    pub height:     u32,
    #[options(help = "fullscreen?")]
    pub fullscreen: bool,

    #[options(help = "Disable monsters")]
    pub no_monsters:   bool,
    #[options(help = "Monsters respawn after being killed")]
    pub respawn_parm:  bool,
    #[options(help = "Monsters move faster")]
    pub fast_parm:     bool,
    #[options(
        help = "Developer mode. F1 saves a screenshot in the current working directory"
    )]
    pub dev_parm:      bool,
    #[options(
        help = "Start a deathmatch game: 1 = classic, 2 = Start a deathmatch 2.0 game.  Weapons do not stay in place and all items respawn after 30 seconds"
    )]
    pub deathmatch:    u8,
    #[options(
        help = "Set the game skill, 1-5 (1: easiest, 5: hardest). A skill of 0 disables all monsters"
    )]
    pub start_skill:   Skill,
    #[options(help = "Select episode")]
    pub start_episode: u32,
    #[options(help = "Select map in episode")]
    pub start_map:     u32,
    pub autostart:     bool,
}

pub fn d_doom_loop(
    mut game: Game,
    mut input: Input,
    mut canvas: Canvas<Window>,
) {
    game.player_in_game[0] = true; // TODO: temporary
    let mut timestep = TimeStep::new();

    'running: loop {
        if !game.running() {
            break 'running;
        }

        try_run_tics(&mut game, &mut input, &mut timestep);
        // TODO: S_UpdateSounds(players[consoleplayer].mo); // move positional sounds
        let surface = Surface::new(320, 200, PixelFormatEnum::RGB555).unwrap();
        let drawer = surface.into_canvas().unwrap();
        // inputs are outside of tic loop?
        d_display(&mut game, &mut input, drawer, &mut canvas);
    }
}

/// D_Display
/// Does a bunch of stuff in Doom...
pub fn d_display(
    game: &mut Game,
    input: &mut Input,
    mut canvas: Canvas<Surface>,
    window: &mut Canvas<Window>,
) {
    //if (gamestate == GS_LEVEL && !automapactive && gametic)
    game.render_player_view(&mut canvas);

    // // menus go directly to the screen
    // M_Drawer();	 // menu is drawn even on top of everything
    // net update does i/o and buildcmds...
    // NetUpdate(); // send out any new accumulation

    // consume the canvas
    i_finish_update(canvas, window);
}

/// Page-flip or blit to screen
pub fn i_finish_update(canvas: Canvas<Surface>, window: &mut Canvas<Window>) {
    //canvas.present();

    let texture_creator = window.texture_creator();
    let t = canvas.into_surface().as_texture(&texture_creator).unwrap();

    window.copy(&t, None, None).unwrap();
    window.present();
}

fn try_run_tics(game: &mut Game, input: &mut Input, timestep: &mut TimeStep) {

    // TODO: net.c starts here
    input.update(); // D_ProcessEvents

    let console_player = game.consoleplayer;
    // net update does i/o and buildcmds...
    // NetUpdate(); // send out any new accumulation

    // temporary block
    game.set_running(!input.get_quit());

    // Network code would update each player slot with incoming TicCmds...
    let cmd = input.tic_events.build_tic_cmd(&input.config);
    game.netcmds[console_player][0] = cmd;

    let tic_events = input.tic_events.clone(); // TODO: Remove when player thinker done

    if tic_events.is_kb_pressed(Scancode::Escape) {
        game.set_running(false);
    }

    // Build tics here?
    timestep.run_this(|time| {
        let time = time * 0.005;
        // TODO: temorary block, remove when tics and player thinker done
        let rot_amnt = 0.15 * time;
        let mv_amnt = 50.0 * time;
        // if tic_events.is_kb_pressed(Scancode::Left) {
        //     game.players[console_player].rotation += rot_amnt;
        // }

        // if tic_events.is_kb_pressed(Scancode::Right) {
        //     game.players[console_player].rotation -= rot_amnt;
        // }

        // if tic_events.is_kb_pressed(Scancode::Up) {
        //     let heading = game.players[console_player].rotation.sin_cos();
        //     game.players[console_player].xy.set_x(
        //         game.players[console_player].xy.x() + heading.1 * mv_amnt,
        //     );
        //     game.players[console_player].xy.set_y(
        //         game.players[console_player].xy.y() + heading.0 * mv_amnt,
        //     );
        // }

        // if tic_events.is_kb_pressed(Scancode::Down) {
        //     let heading = game.players[console_player].rotation.sin_cos();
        //     game.players[console_player].xy.set_x(
        //         game.players[console_player].xy.x() - heading.1 * mv_amnt,
        //     );
        //     game.players[console_player].xy.set_y(
        //         game.players[console_player].xy.y() - heading.0 * mv_amnt,
        //     );
        // }

        // G_Ticker
        game.ticker();
    });
}
