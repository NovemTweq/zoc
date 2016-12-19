use std::default::{Default};
use std::rc::{Rc};
use types::{Size2};
use internal_state::{InternalState};
use game_state::{GameState};
use map::{Map, Terrain, distance};
use fov::{fov, simple_fov};
use db::{Db};
use unit::{Unit, UnitType};
use ::{CoreEvent, PlayerId, MapPos, ExactPos, ObjectClass};

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub enum TileVisibility {
    No,
    // Bad,
    Normal,
    Excellent,
}

impl Default for TileVisibility {
    fn default() -> Self { TileVisibility::No }
}

fn fov_unit(
    db: &Db,
    state: &InternalState,
    fow: &mut Map<TileVisibility>,
    unit: &Unit,
) {
    fov_unit_in_pos(db, state, fow, unit, unit.pos.map_pos);
}

fn fov_unit_in_pos(
    db: &Db,
    state: &InternalState,
    fow: &mut Map<TileVisibility>,
    unit: &Unit,
    origin: MapPos,
) {
    assert!(unit.is_alive);
    let unit_type = db.unit_type(unit.type_id);
    let range = unit_type.los_range;
    let f = if unit_type.is_air {
        simple_fov
    } else {
        fov
    };
    f(
        state,
        origin,
        range,
        &mut |pos| {
            let vis = calc_visibility(state, unit_type, origin, pos);
            if vis > *fow.tile_mut(pos) {
                *fow.tile_mut(pos) = vis;
            }
        },
    );
}

fn calc_visibility<S: GameState>(
    state: &S,
    unit_type: &UnitType,
    origin: MapPos,
    pos: MapPos,
) -> TileVisibility {
    let distance = distance(origin, pos);
    if distance > unit_type.los_range {
        return TileVisibility::No;
    }
    if distance <= unit_type.cover_los_range {
        return TileVisibility::Excellent;
    }
    let mut vis = match *state.map().tile(pos) {
        Terrain::City | Terrain::Trees => TileVisibility::Normal,
        Terrain::Plain | Terrain::Water => TileVisibility::Excellent,
    };
    for object in state.objects_at(pos) {
        match object.class {
            // TODO: Removed Terrain::City and Terrain::Trees, use Smoke-like objects in logic
            ObjectClass::Building | ObjectClass::Smoke => {
                vis = TileVisibility::Normal;
            }
            ObjectClass::Road |
            ObjectClass::ReinforcementSector => {},
        }
    }
    vis
}

/// Fog of War
#[derive(Clone, Debug)]
pub struct Fow {
    map: Map<TileVisibility>,
    player_id: PlayerId,
    db: Rc<Db>,
}

impl Fow {
    pub fn new(db: Rc<Db>, map_size: Size2, player_id: PlayerId) -> Fow {
        Fow {
            map: Map::new(map_size),
            player_id: player_id,
            db: db,
        }
    }

    pub fn is_tile_visible(&self, pos: MapPos) -> bool {
        match *self.map.tile(pos) {
            TileVisibility::Excellent |
            TileVisibility::Normal => true,
            TileVisibility::No => false,
        }
    }

    fn check_terrain_visibility(&self, unit_type: &UnitType, pos: MapPos) -> bool {
        match *self.map.tile(pos) {
            TileVisibility::Excellent => true,
            TileVisibility::Normal => !unit_type.is_infantry,
            TileVisibility::No => false,
        }
    }

    pub fn is_visible<S: GameState>(
        &self,
        state: &S,
        unit: &Unit,
        pos: ExactPos,
    ) -> bool {
        // TODO is_transporter_or_attached
        for (_, other_unit) in state.units() {
            if let Some(passenger_id) = other_unit.passenger_id {
                if passenger_id == unit.id && other_unit.pos == pos {
                    return false;
                }
            }
        }
        let unit_type = self.db.unit_type(unit.type_id);
        if unit_type.is_air {
            // TODO: туповатая проверка
            // так воздушный юнит может пропасть из видимости
            // просто если увести наблюдателя.
            //
            // лучше запилить второй слой в тумане войны
            for (_, enemy_unit) in state.units() {
                if enemy_unit.player_id == unit.player_id {
                    continue;
                }
                let enemy_unit_type = self.db.unit_type(enemy_unit.type_id);
                let distance = distance(pos.map_pos, enemy_unit.pos.map_pos);
                if distance <= enemy_unit_type.los_range {
                    return true;
                }
            }
        }
        self.check_terrain_visibility(unit_type, pos.map_pos)
    }

    fn clear(&mut self) {
        for pos in self.map.get_iter() {
            *self.map.tile_mut(pos) = TileVisibility::No;
        }
    }

    fn reset(&mut self, state: &InternalState) {
        self.clear();
        for (_, unit) in state.units() {
            if unit.player_id == self.player_id && unit.is_alive {
                fov_unit(&self.db, state, &mut self.map, unit);
            }
        }
    }

    pub fn apply_event(
        &mut self,
        state: &InternalState,
        event: &CoreEvent,
    ) {
        match *event {
            CoreEvent::Move{unit_id, to, ..} => {
                let unit = state.unit(unit_id);
                if unit.player_id == self.player_id {
                    fov_unit_in_pos(
                        &self.db, state, &mut self.map, unit, to.map_pos);
                }
            },
            CoreEvent::EndTurn{new_id, ..} => {
                if self.player_id == new_id {
                    self.reset(state);
                }
            },
            CoreEvent::CreateUnit{ref unit_info} => {
                let unit = state.unit(unit_info.unit_id);
                if self.player_id == unit_info.player_id {
                    fov_unit(&self.db, state, &mut self.map, unit);
                }
            },
            CoreEvent::AttackUnit{ref attack_info} => {
                if let Some(attacker_id) = attack_info.attacker_id {
                    if !attack_info.is_ambush {
                        let pos = state.unit(attacker_id).pos;
                        // TODO: do not give away all units in this tile!
                        *self.map.tile_mut(pos) = TileVisibility::Excellent;
                    }
                }
            },
            CoreEvent::UnloadUnit{ref unit_info, ..} => {
                if self.player_id == unit_info.player_id {
                    let unit = state.unit(unit_info.unit_id);
                    let pos = unit_info.pos.map_pos;
                    fov_unit_in_pos(&self.db, state, &mut self.map, unit, pos);
                }
            },
            CoreEvent::Spotted{..} |
            CoreEvent::ShowUnit{..} |
            CoreEvent::HideUnit{..} |
            CoreEvent::LoadUnit{..} |
            CoreEvent::Attach{..} |
            CoreEvent::Detach{..} |
            CoreEvent::SetReactionFireMode{..} |
            CoreEvent::SectorOwnerChanged{..} |
            CoreEvent::Smoke{..} |
            CoreEvent::RemoveSmoke{..} |
            CoreEvent::VictoryPoint{..} => {},
        }
    }
}

#[derive(Clone, Debug)]
pub struct FakeFow;

static FAKE_FOW: FakeFow = FakeFow;

pub fn fake_fow() -> &'static FakeFow {
    &FAKE_FOW
}

pub trait FogOfWar: Clone {
    fn is_visible<S: GameState>(
        &self,
        state: &S,
        unit: &Unit,
        pos: ExactPos,
    ) -> bool;
}

impl FogOfWar for FakeFow {
    fn is_visible<S: GameState>(
        &self,
        _: &S,
        _: &Unit,
        _: ExactPos,
    ) -> bool {
        true
    }
}

impl FogOfWar for Fow {
    fn is_visible<S: GameState>(
        &self,
        state: &S,
        unit: &Unit,
        pos: ExactPos,
    ) -> bool {
        self.is_visible(state, unit, pos)
    }
}
