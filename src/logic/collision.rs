use crate::graphics::{animation::AnimationState, tiles::Tilemap};
use crate::logic::{
    state::{EntityAnims, EntityState},
    types::{Rect, Vec2i},
};

fn rect_touching(r1: &Rect, r2: &Rect) -> bool {
    // r1 left is left of r2 right
    r1.x <= r2.x+r2.w as i32 &&
        // r2 left is left of r1 right
        r2.x <= r1.x+r1.w as i32 &&
        // those two conditions handle the x axis overlap;
        // the next two do the same for the y axis:
        r1.y <= r2.y+r2.h as i32 &&
        r2.y <= r1.y+r1.h as i32
}

fn rect_displacement(r1: &Rect, r2: &Rect) -> Option<(i32, i32)> {
    // Draw this out on paper to double check, but these quantities
    // will both be positive exactly when the conditions in rect_touching are true.
    let x_overlap = (r1.x + r1.w as i32).min(r2.x + r2.w as i32) - r1.x.max(r2.x);
    let y_overlap = (r1.y + r1.h as i32).min(r2.y + r2.h as i32) - r1.y.max(r2.y);
    if x_overlap >= 0 && y_overlap >= 0 {
        // This will return the magnitude of overlap in each axis.
        Some((x_overlap, y_overlap))
    } else {
        None
    }
}

pub struct WallContact {
    wall_rect: Rect,
    entity_id: usize,
    contact: (i32, i32),
}

pub fn gather_contacts(
    walls: &Vec<Tilemap>,
    entity: &Rect,
    entity_id: usize,
    contacts: &mut Vec<WallContact>,
) -> bool {
    let mut game_over = false;
    // Find four courners of the entity and check for collision
    // Note: the entity has to be smaller than the tiles
    let corners: Vec<Vec2i> = vec![
        Vec2i(entity.x, entity.y),
        Vec2i(entity.x + entity.w as i32, entity.y),
        Vec2i(entity.x, entity.y + entity.h as i32),
        Vec2i(entity.x + entity.w as i32, entity.y + entity.h as i32),
    ];

    // Find associated tilemap(s)
    for corner in corners {
        for map in walls {
            // In a specific example, you can probably index with a calculation
            if map.contains(corner) && (map.tile_at(corner).solid || map.tile_at(corner).triangle) {
                let tile = map.tile_at(corner);

                let wall_rect = map.get_tile_rect(corner);

                // Check the traingle collision
                // println!("{}, {}", tile.triangle, triangle_collision(corner, wall_rect));
                if tile.triangle {
                    if triangle_collision(corner, wall_rect) {
                        game_over = true;
                    }
                    continue;
                }
                match rect_displacement(entity, &wall_rect) {
                    Some(contact) => {
                        // println!("{}, {}", contact.0, contact.1);
                        contacts.push(WallContact {
                            wall_rect,
                            entity_id,
                            contact,
                        })
                    }
                    None => {}
                }
            }
        }
    }
    game_over
}

pub fn killed(entity: &Rect, enemies: &[Rect]) -> bool {
    for enemy in enemies {
        if rect_touching(entity, enemy) {
            match rect_displacement(entity, enemy) {
                Some(_) => {
                    return true;
                }
                None => {}
            }
        }
    }
    false
}

pub fn restitute(
    positions: &mut Vec<Vec2i>,
    states: &mut Vec<EntityState>,
    vels: &mut Vec<Vec2i>,
    sizes: &Vec<(usize, usize)>,
    contacts: &mut Vec<WallContact>,
    anims: &mut Vec<AnimationState>,
    ent_anims: &EntityAnims,
) -> bool {
    let mut game_over = false;

    for contact in contacts {
        let wall_rect = contact.wall_rect;
        let min_overlap = contact.contact.0.min(contact.contact.1);
        let mut pos = positions.get_mut(contact.entity_id).unwrap();
        let mut vel = vels.get_mut(contact.entity_id).unwrap();
        let state = states.get_mut(contact.entity_id).unwrap();
        let anim = anims.get_mut(contact.entity_id).unwrap();

        // Check if the contact is still overlapping
        let sprite_rect = Rect {
            x: pos.0,
            y: pos.1,
            w: sizes[contact.entity_id].0 as u16,
            h: sizes[contact.entity_id].1 as u16,
        };
        if rect_displacement(&wall_rect, &sprite_rect).is_none() {
            continue;
        }

        match min_overlap {
            overlap if overlap == 0 => {
                continue;
            }
            overlap if overlap == contact.contact.0 => {
                if min_overlap > sizes[contact.entity_id].1 as i32 / 2 {
                    // pos.1 -= contact.contact.1;
                    game_over = true;
                }
                pos.0 += (pos.0 - wall_rect.x).signum() * min_overlap;
            }
            _ => {
                if pos.1 - wall_rect.y > 0 {
                    game_over = true;
                }
                pos.1 += (pos.1 - wall_rect.y).signum() * min_overlap;
            }
        }
        if *state == EntityState::Falling {
            *state = EntityState::Landing;
            *anim = ent_anims.landing.start();
        };
        // vel.0 = if contact.contact.0 >= 0 {0} else {vel.0};
        vel.1 = if contact.contact.0 >= 0 { 0 } else { vel.1 };
    }
    game_over
}

fn triangle_collision(p: Vec2i, Rect { x, y, w, h }: Rect) -> bool {
    // Tiles must have even size for this to work
    let a = Vec2i(x + (w as i32) / 2, y);
    let b = Vec2i(x, y + h as i32);
    let c = Vec2i(x + w as i32, y + h as i32);

    let ab = Vec2i(b.0 - a.0, b.1 - a.1);
    let bc = Vec2i(c.0 - b.0, c.1 - b.1);
    let ca = Vec2i(a.0 - c.0, a.1 - c.1);

    let ap = Vec2i(p.0 - a.0, p.1 - a.1);
    let bp = Vec2i(p.0 - b.0, p.1 - b.1);
    let cp = Vec2i(p.0 - c.0, p.1 - c.1);

    let cp1 = cross_product(ab, ap);
    let cp2 = cross_product(bc, bp);
    let cp3 = cross_product(ca, cp);
    cp1.signum() == cp2.signum() && cp1.signum() == cp3.signum()
}

fn cross_product(Vec2i(x1, y1): Vec2i, Vec2i(x2, y2): Vec2i) -> i32 {
    x1 * y2 - x2 * y1
}
