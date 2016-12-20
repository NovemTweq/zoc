use std::collections::{HashMap};
use std::rc::{Rc};
use db::{Db};
use unit::{Unit};
use map::{Map, Terrain};
use internal_state::{InternalState};
use game_state::{GameState, GameStateMut, UnitIter};
use fow::{FakeFow, fake_fow};
use ::{
    CoreEvent,
    PlayerId,
    UnitId,
    ObjectId,
    Object,
    Score,
    Sector,
    SectorId,
    Options,
    ReinforcementPoints,
};

#[derive(Clone, Debug)]
pub struct FullState {
    state: InternalState,
}

impl FullState {
    pub fn new(db: Rc<Db>, options: &Options) -> FullState {
        FullState {
            state: InternalState::new(db, options),
        }
    }

    pub fn inner(&self) -> &InternalState {
        &self.state
    }
}

impl GameState for FullState {
    type Fow = FakeFow;

    fn units<'a>(&'a self) -> UnitIter<'a, Self::Fow> {
        UnitIter {
            iter: self.state.raw_units(),
            fow: fake_fow(),
        }
    }

    fn unit_opt(&self, id: UnitId) -> Option<&Unit> {
        self.state.unit_opt(id)
    }

    fn objects(&self) -> &HashMap<ObjectId, Object> {
        self.state.objects()
    }

    fn map(&self) -> &Map<Terrain> {
        self.state.map()
    }

    fn sectors(&self) -> &HashMap<SectorId, Sector> {
        self.state.sectors()
    }

    fn score(&self) -> &HashMap<PlayerId, Score> {
        self.state.score()
    }

    fn reinforcement_points(&self) -> &HashMap<PlayerId, ReinforcementPoints> {
        self.state.reinforcement_points()
    }
}

impl GameStateMut for FullState {
    fn apply_event(&mut self, event: &CoreEvent) {
        self.state.apply_event(event);
    }
}
