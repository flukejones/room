//!	Movement, collision handling.
//!	Shooting and aiming.
use glam::Vec2;

use crate::flags::LineDefFlags;
use crate::level_data::level::Level;
use crate::level_data::map_data::MapData;
use crate::level_data::map_defs::{BBox, LineDef, SubSector};
use crate::p_local::MAXRADIUS;
use crate::p_map_object::{MapObject, MapObjectFlag, MAXMOVE};
use crate::p_map_util::{
    circle_to_seg_intersect, unit_vec_from, LineContact, PortalZ,
};
use crate::DPtr;

const MAXSPECIALCROSS: i32 = 8;

/// The pupose of this struct is to record the highest and lowest points in a
/// subsector. When a mob crosses a seg it may be between floor/ceiling heights.
#[derive(Default)]
pub(crate) struct SubSectorMinMax {
    tmflags:     u32,
    /// If "floatok" true, move would be ok
    /// if within "tmfloorz - tmceilingz".
    floatok:     bool,
    min_floor_z: f32,
    max_ceil_z:  f32,
    max_dropoff: f32,
    spec_hits:   Vec<DPtr<LineDef>>,
}

impl MapObject {
    fn get_contacts_in_ssects(
        &mut self,
        subsect: &SubSector,
        ctrl: &mut SubSectorMinMax,
        map_data: &MapData,
    ) -> Vec<LineContact> {
        let mut contacts: Vec<LineContact> = Vec::new();
        // TODO: figure out a better way to get all segs in vicinity;
        let segs = map_data.get_segments();
        for seg in &segs[subsect.start_seg as usize
            ..(subsect.start_seg + subsect.seg_count) as usize]
        {
            //for seg in segs.iter() {
            if let Some(contact) = self.pit_check_line(ctrl, &seg.linedef) {
                contacts.push(contact);
            }
        }
        contacts
    }

    fn resolve_contacts(&mut self, contacts: &[LineContact]) {
        for contact in contacts.iter() {
            self.momxy -= contact.normal * contact.penetration;
        }
    }

    /// P_TryMove, merged with P_CheckPosition and using a more verbose/modern collision
    pub fn p_try_move(&mut self, level: &mut Level) {
        // P_CrossSpecialLine
        level.mobj_ctrl.floatok = false;

        let ctrl = &mut level.mobj_ctrl;
        // TODO: ceilingline = NULL;

        // First sector is always the one we are in
        let curr_subsect =
            level.map_data.point_in_subsector(&(self.xy + self.momxy));
        ctrl.min_floor_z = curr_subsect.sector.floorheight;
        ctrl.max_dropoff = curr_subsect.sector.floorheight;
        ctrl.max_ceil_z = curr_subsect.sector.ceilingheight;

        // TODO: validcount++;??? There's like, two places in the p_map.c file
        if ctrl.tmflags & MapObjectFlag::MF_NOCLIP as u32 != 0 {
            return;
        }

        // Check things first, possibly picking things up.
        // TODO: P_BlockThingsIterator, PIT_CheckThing

        // This is effectively P_BlockLinesIterator, PIT_CheckLine
        let mut blocked = false;
        let contacts =
            self.get_contacts_in_ssects(&curr_subsect, ctrl, &level.map_data);

        // TODO: find the most suitable contact to move with (wall sliding)
        if !contacts.is_empty() {
            blocked = true;
            self.momxy = contacts[0].slide_dir
                * contacts[0].angle_delta
                * self.momxy.length();

            let contacts = self.get_contacts_in_ssects(
                &curr_subsect,
                ctrl,
                &level.map_data,
            );
            self.resolve_contacts(&contacts);
        }

        self.xy += self.momxy;
        if !blocked {
            self.floorz = ctrl.min_floor_z;
            self.ceilingz = ctrl.max_ceil_z;
        }

        // TODO: if any special lines were hit, do the effect
        // if (!(thing->flags & (MF_TELEPORT | MF_NOCLIP)))
        // {
        //     while (numspechit--)
        //     {
        //         // see if the line was crossed
        //         ld = spechit[numspechit];
        //         side = P_PointOnLineSide(thing->x, thing->y, ld);
        //         oldside = P_PointOnLineSide(oldx, oldy, ld);
        //         if (side != oldside)
        //         {
        //             if (ld->special)
        //                 P_CrossSpecialLine(ld - lines, oldside, thing);
        //         }
        //     }
        // }
    }

    /// PIT_CheckLine
    /// Adjusts tmfloorz and tmceilingz as lines are contacted, if
    /// penetration with a line is detected then the pen distance is returned
    fn pit_check_line(
        &mut self,
        ctrl: &mut SubSectorMinMax,
        ld: &LineDef,
    ) -> Option<LineContact> {
        if ld.point_on_side(&self.xy) == 1 {
            return None;
        }

        if let Some(contact) = circle_to_seg_intersect(
            self.xy,
            self.momxy,
            self.radius,
            *ld.v1,
            *ld.v2,
        ) {
            // TODO: really need to check the lines of the subsector on the
            //  on the other side of the contact too

            if ld.backsector.is_none() {
                // one-sided line
                return Some(contact);
            }

            // Flag checks
            // TODO: can we move these up a call?
            if self.flags & MapObjectFlag::MF_MISSILE as u32 == 0 {
                if ld.flags & LineDefFlags::Blocking as i16 != 0 {
                    return Some(contact); // explicitly blocking everything
                }

                if self.player.is_none()
                    && ld.flags & LineDefFlags::BlockMonsters as i16 != 0
                {
                    return Some(contact); // block monsters only
                }
            } else if self.flags & MapObjectFlag::MF_MISSILE as u32 != 0 {
                return Some(contact);
            }

            // Find the smallest/largest etc if group of line hits
            let portal = PortalZ::new(ld);
            if portal.top_z < ctrl.max_ceil_z {
                ctrl.max_ceil_z = portal.top_z;
                // TODO: ceilingline = ld;
            }
            if portal.bottom_z > ctrl.min_floor_z {
                ctrl.min_floor_z = portal.bottom_z;
            }
            if portal.low_point < ctrl.max_dropoff {
                ctrl.max_dropoff = portal.low_point;
            }

            if ld.special != 0 {
                ctrl.spec_hits.push(DPtr::new(ld));
            }

            if portal.bottom_z - self.z > 24.0 && portal.bottom_z > self.z {
                return Some(contact);
            }

            // Line crossed, we might be colliding a nearby line
            if let Some(back) = &ld.backsector {
                for line in back.lines.iter() {
                    if *line.v1 == *ld.v1 && *line.v2 == *ld.v2
                        || *line.v1 == *ld.v2 && *line.v2 == *ld.v1
                    {
                        continue;
                    }
                    if let Some(contact) = self.pit_check_line(ctrl, line) {
                        return Some(contact);
                    }
                }
            }
        }
        None
    }
}

/// P_RadiusAttack
/// Source is the creature that caused the explosion at spot.
pub(crate) fn p_radius_attack(
    spot: &mut MapObject,
    source: &mut MapObject,
    damage: f32,
) {
    let dist = damage + MAXRADIUS;
    unimplemented!()
    // // origin of block level is bmaporgx and bmaporgy
    // let yh = (spot.xy.y() + dist - bmaporgy) >> MAPBLOCKSHIFT;
    // let yl = (spot.xy.y() - dist - bmaporgy) >> MAPBLOCKSHIFT;
    // let xh = (spot.xy.x() + dist - bmaporgx) >> MAPBLOCKSHIFT;
    // let xl = (spot.xy.x() - dist - bmaporgx) >> MAPBLOCKSHIFT;
    // bombspot = spot;
    // bombsource = source;
    // bombdamage = damage;

    // for (y = yl; y <= yh; y++)
    // for (x = xl; x <= xh; x++)
    // P_BlockThingsIterator(x, y, PIT_RadiusAttack);
}
