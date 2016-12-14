use std::collections::{HashMap};
use std::collections::hash_map;
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use ::{
    CoreEvent,
    UnitId,
    ObjectId,
    Object,
    MapPos,
    Sector,
    SectorId,
    PlayerId,
    Score,
    ReinforcementPoints,
};

#[derive(Clone)]
pub struct ObjectsAtIter<'a> {
    it: hash_map::Iter<'a, ObjectId, Object>,
    pos: MapPos,
}

impl<'a> ObjectsAtIter<'a> {
    pub fn new(objects: &HashMap<ObjectId, Object>, pos: MapPos) -> ObjectsAtIter {
        ObjectsAtIter{it: objects.iter(), pos: pos}
    }
}

impl<'a> Iterator for ObjectsAtIter<'a> {
    type Item = &'a Object;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((_, object)) = self.it.next() {
            for map_pos in object.pos.map_pos_iter() {
                if self.pos == map_pos {
                    return Some(object);
                }
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct UnitsAtIter<'a> {
    it: hash_map::Iter<'a, UnitId, Unit>,
    pos: MapPos,
}

impl<'a> Iterator for UnitsAtIter<'a> {
    type Item = &'a Unit;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((_, unit)) = self.it.next() {
            if self.pos == unit.pos.map_pos {
                return Some(unit);
            }
        }
        None
    }
}

pub trait GameState {
    fn map(&self) -> &Map<Terrain>;

    fn units(&self) -> hash_map::Iter<UnitId, Unit>;

    /*
    fn units2(&self) -> hash_map::Iter<UnitId, Unit> {
        self.units().iter()
    }
    */

    fn unit_opt(&self, id: UnitId) -> Option<&Unit>;

    fn objects(&self) -> &HashMap<ObjectId, Object>;
    fn sectors(&self) -> &HashMap<SectorId, Sector>;
    fn score(&self) -> &HashMap<PlayerId, Score>;
    fn reinforcement_points(&self) -> &HashMap<PlayerId, ReinforcementPoints>;

    fn unit(&self, id: UnitId) -> &Unit {
        self.unit_opt(id).unwrap()
    }

    fn units_at(&self, pos: MapPos) -> UnitsAtIter {
        UnitsAtIter{it: self.units(), pos: pos}
    }

    fn objects_at(&self, pos: MapPos) -> ObjectsAtIter {
        ObjectsAtIter::new(self.objects(), pos)
    }
}

pub trait GameStateMut: GameState {
    fn apply_event(&mut self, db: &Db, event: &CoreEvent);
}
