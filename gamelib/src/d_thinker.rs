use crate::p_map_object::MapObject;
use crate::p_player_sprite::PspDef;
use crate::player::Player;
use std::fmt;
use std::ptr::null_mut;

// TODO: Split thinkers and things in to MapObject, Lights, Movers, Player,
//  where Movers contains all level structure changing things like doors/platforms

/// Thinkers *must* be contained in a structure that has **stable** memory locations.
/// In Doom this is managed by Doom's custom allocator, where each location in memory
/// also has a pointer to the locations 'owner'. When Doom does a defrag or any op
/// that moves memory locations it also runs through the owners and updates their
/// pointers. This isn't done in the Rust version.
///
/// Another way to manager Thinkers in a volatile container like a Vec is to use `self.function`
/// to mark for removal (same as Doom), then iterate over the container and only
/// run thinkers not marked for removal, then remove marked thinkers after cycle.
/// This method would have a big impact on iter speed though as there may be many
/// 'dead' thinkers.
///
/// Inserting the `Thinker` in to the game is done in p_tick.c with `P_RunThinkers`.
///
/// On Drop the thinker will unlink itself from whereever it is in the link chain.
///
///  State should live in MapObject. State.action and Think.function are two
///  different functions
///
/// The LinkedList style serves to give the Objects a way to find the next/prev of
/// its neighbours and more, without having to pass in a ref to the Thinker container
#[derive(Debug)]
pub struct Thinker<'t, T: 't> {
    pub prev:     *mut Thinker<'t, T>,
    pub next:     *mut Thinker<'t, T>,
    pub obj:      T,
    /// The `Thinker` function to run, this function typically also runs a `State`
    /// function on the Object. The `State` function may then require access to
    /// the `Thinker` to change/remove the thinker funciton.
    // TODO: maybe make this take the Thinker as arg. Easily done if the Thinker then contains
    //  only one struct type for things
    pub function: ActionF,
}

impl<'t, T> Thinker<'t, T> {
    pub fn new(obj: T) -> Thinker<'t, T> {
        Thinker {
            prev: null_mut(),
            next: null_mut(),
            obj,
            function: ActionF::Acv,
        }
    }

    pub fn unlink(&mut self) {
        if !self.prev.is_null() && !self.next.is_null() {
            let prev = unsafe { &mut *self.prev };
            let next = unsafe { &mut *self.next };
            prev.next = next;
            next.prev = prev;
        } else if !self.prev.is_null() && self.next.is_null() {
            // Only linked to previous, so unlink prev to this thinker
            unsafe {
                (*self.prev).next = null_mut();
            }
        } else if self.prev.is_null() && !self.next.is_null() {
            // Only linked to next, so unlink next to this thinker
            unsafe {
                (*self.next).prev = null_mut();
            }
        }
    }
}

impl<'t, T> Drop for Thinker<'t, T> {
    fn drop(&mut self) {
        // if this thinker has links in both directions then the thinkers at those
        // ends must be linked to this thinker, so we need to unlink those from
        // this thinker, and link them together
        self.unlink();
    }
}

/// Enum of function callbacks
///
/// Similar to `actionf_t` in d_think.h. `ObjectBase` is required because we need to wrap the
/// various different args *because* unlike C we can't rely on function arg casts. Use of `Any`
/// could be done, but it introduces overhead at runtime.
#[derive(Clone)]
pub enum ActionF {
    Acv,
    /// NULL thinker, used to tell the thinker runner to remove the thinker from list
    Acp1(*const dyn Fn(&mut ObjectBase)),
    /// Called in the MapObject state setter
    Acp2(*const dyn Fn(&mut ObjectBase, &mut PspDef)), // P_SetPsprite runs this
    // TODO: Could do something like
    //  - MabObject(*const dyn Fn(&mut MapObject)),
    //  - Player(*const dyn Fn(&mut Player, &mut PspDef)),
    //  etc. Thinker would need to be separated and specialied to the 3 different map
    //  things (light/mover/thing). Full list is:
    //  - ceiling_t
    //  - vldoor_t
    //  - floormove_t
    //  - plat_t
    //  - fireflicker_t
    //  - lightflash_t
    //  - strobe_t
    //  - glow_t
    //  - mobj_t
    /// No action, used in place of Acp1(&null)
    None,
}

impl ActionF {
    pub fn do_action1(&self, object: &mut ObjectBase) {
        match self {
            ActionF::Acp1(f) => unsafe { (**f)(object) },
            _ => {}
        }
    }

    pub fn do_action2(&self, object1: &mut ObjectBase, object2: &mut PspDef) {
        match self {
            ActionF::Acp2(f) => unsafe { (**f)(object1, object2) },
            _ => {}
        }
    }
}

impl fmt::Debug for ActionF {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Action").finish()
    }
}

/// Container of all possible map object types.
///
/// **Moving segs/sectors**
/// - ceiling_t
/// - vldoor_t
/// - floormove_t
/// - plat_t
///
/// **Level lights**
/// - fireflicker_t
/// - lightflash_t
/// - strobe_t
/// - glow_t
///
/// **Items like Health, Ammo, Lamps, corpses, demons, player etc**
/// - mobj_t (MapObject)
///
/// All of these object types have an associated thinker function, `ceiling_t` uses a
/// thinker function `T_MoveCeiling()`
#[derive(Debug)]
pub enum ObjectBase<'m> {
    MapObject(MapObject<'m>),
    Player(Player<'m>),
}

impl<'m> ObjectBase<'m> {
    pub fn get_map_obj(&self) -> Option<&'m MapObject> {
        match self {
            ObjectBase::MapObject(m) => Some(&m),
            _ => None,
        }
    }

    pub fn get_mut_map_obj(&mut self) -> Option<&'m mut MapObject> {
        match self {
            ObjectBase::MapObject(ref mut m) => Some(m),
            _ => None,
        }
    }

    pub fn get_player(&self) -> Option<&'m Player> {
        match self {
            ObjectBase::Player(m) => Some(&m),
            _ => None,
        }
    }

    pub fn get_mut_player(&mut self) -> Option<&'m mut Player> {
        match self {
            ObjectBase::Player(ref mut m) => Some(m),
            _ => None,
        }
    }
}
