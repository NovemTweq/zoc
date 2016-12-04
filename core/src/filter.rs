use std::collections::{HashMap, HashSet};
use internal_state::{InternalState};
use game_state::{GameState};
use unit::{Unit};
use db::{Db};
use fow::{Fow};
use ::{
    CoreEvent,
    AttackInfo,
    UnitInfo,
    UnitId,
    PlayerId,
    MoveMode,
    MovePoints,
    unit_to_info,
};

pub fn get_visible_enemies(
    db: &Db,
    state: &InternalState,
    fow: &Fow,
    player_id: PlayerId,
) -> HashSet<UnitId> {
    let mut visible_enemies = HashSet::new();
    for (&id, unit) in state.units() {
        if unit.player_id != player_id
            && fow.is_visible(db, state, unit, unit.pos)
        {
            visible_enemies.insert(id);
        }
    }
    visible_enemies
}

pub fn show_or_hide_passive_enemies(
    units: &HashMap<UnitId, Unit>,
    active_unit_ids: &HashSet<UnitId>,
    old: &HashSet<UnitId>,
    new: &HashSet<UnitId>,
) -> Vec<CoreEvent> {
    let mut events = Vec::new();
    let located_units = new.difference(old);
    for id in located_units {
        if active_unit_ids.contains(id) {
            continue;
        }
        let unit = units.get(id).expect("Can`t find unit");
        events.push(CoreEvent::ShowUnit {
            unit_info: unit_to_info(unit),
        });
    }
    let lost_units = old.difference(new);
    for &id in lost_units {
        if active_unit_ids.contains(&id) {
            continue;
        }
        events.push(CoreEvent::HideUnit{unit_id: id});
    }
    events
}

// TODO: join state and fow into TmpPartialState
pub fn filter_events(
    db: &Db,
    state: &InternalState,
    player_id: PlayerId,
    fow: &Fow,
    event: &CoreEvent,
) -> (Vec<CoreEvent>, HashSet<UnitId>) {
    let mut active_unit_ids = HashSet::new();
    let mut events = vec![];
    match *event {
        CoreEvent::Move{unit_id, from, to, ..} => {
            let unit = state.unit(unit_id);
            if unit.player_id == player_id {
                events.push(event.clone());
            } else {
                let prev_vis = fow.is_visible(db, state, unit, from);
                let next_vis = fow.is_visible(db, state, unit, to);
                if !prev_vis && next_vis {
                    events.push(CoreEvent::ShowUnit {
                        unit_info: UnitInfo {
                            pos: from,
                            .. unit_to_info(unit)
                        },
                    });
                    if let Some(attached_unit_id) = unit.attached_unit_id {
                        active_unit_ids.insert(attached_unit_id);
                        let attached_unit = state.unit(attached_unit_id);
                        events.push(CoreEvent::ShowUnit {
                            unit_info: UnitInfo {
                                pos: from,
                                .. unit_to_info(attached_unit)
                            },
                        });
                    }
                }
                if prev_vis || next_vis {
                    events.push(event.clone());
                }
                if prev_vis && !next_vis {
                    events.push(CoreEvent::HideUnit {
                        unit_id: unit.id,
                    });
                }
                active_unit_ids.insert(unit_id);
            }
        },
        CoreEvent::CreateUnit{ref unit_info} => {
            let unit = state.unit(unit_info.unit_id);
            if player_id == unit_info.player_id
                || fow.is_visible(db, state, unit, unit_info.pos)
            {
                events.push(event.clone());
                active_unit_ids.insert(unit_info.unit_id);
            }
        },
        CoreEvent::AttackUnit{ref attack_info} => {
            let attacker_id = attack_info.attacker_id
                .expect("Core must know about everything");
            let attacker = state.unit(attacker_id);
            if player_id != attacker.player_id && !attack_info.is_ambush {
                // show attacker if this is not ambush
                let attacker = state.unit(attacker_id);
                if !fow.is_visible(db, state, attacker, attacker.pos) {
                    events.push(CoreEvent::ShowUnit {
                        unit_info: unit_to_info(attacker),
                    });
                }
                active_unit_ids.insert(attacker_id);
            }
            active_unit_ids.insert(attack_info.defender_id); // if defender is killed
            let is_attacker_visible = player_id == attacker.player_id
                || !attack_info.is_ambush;
            let attack_info = AttackInfo {
                attacker_id: if is_attacker_visible {
                    Some(attacker_id)
                } else {
                    None
                },
                .. attack_info.clone()
            };
            events.push(CoreEvent::AttackUnit{attack_info: attack_info});
        },
        CoreEvent::ShowUnit{..} => panic!(),
        CoreEvent::HideUnit{..} => panic!(),
        CoreEvent::LoadUnit{passenger_id, from, to, transporter_id} => {
            // TODO: тут, кстати, тоже было бы хорошо в движение переделывать событие
            // Тогда и Option какой-то там можно было бы убрать
            let passenger = state.unit(passenger_id);
            let transporter = state.unit(transporter_id.unwrap());
            let is_transporter_vis = fow.is_visible(
                db, state, transporter, transporter.pos);
            let is_passenger_vis = fow.is_visible(
                db, state, passenger, from);
            if passenger.player_id == player_id {
                events.push(event.clone());
            } else if is_passenger_vis || is_transporter_vis {
                if !fow.is_visible(db, state, passenger, from) {
                    events.push(CoreEvent::ShowUnit {
                        unit_info: UnitInfo {
                            pos: from,
                            .. unit_to_info(passenger)
                        },
                    });
                }
                let filtered_transporter_id = if is_transporter_vis {
                    transporter_id
                } else {
                    None
                };
                events.push(CoreEvent::LoadUnit {
                    transporter_id: filtered_transporter_id,
                    passenger_id: passenger_id,
                    from: from,
                    to: to,
                });
                active_unit_ids.insert(passenger_id);
            }
        },
        CoreEvent::UnloadUnit{ref unit_info, transporter_id, from, to} => {
            // TODO: тут, кстати, тоже было бы хорошо в движение переделывать событие
            active_unit_ids.insert(unit_info.unit_id);
            let passenger = state.unit(unit_info.unit_id);
            let transporter = state.unit(transporter_id.unwrap());
            let is_transporter_vis = fow.is_visible(
                db, state, transporter, from);
            let is_passenger_vis = fow.is_visible(
                db, state, passenger, to);
            if passenger.player_id == player_id {
                events.push(event.clone());
            } else if is_passenger_vis || is_transporter_vis {
                let filtered_transporter_id = if is_transporter_vis {
                    transporter_id
                } else {
                    None
                };
                events.push(CoreEvent::UnloadUnit {
                    transporter_id: filtered_transporter_id,
                    unit_info: unit_info.clone(),
                    from: from,
                    to: to,
                });
                if !is_passenger_vis {
                    events.push(CoreEvent::HideUnit {
                        unit_id: passenger.id,
                    });
                }
            }
        },
        CoreEvent::Attach{transporter_id, attached_unit_id, from, to} => {
            let transporter = state.unit(transporter_id);
            if transporter.player_id == player_id {
                events.push(event.clone())
            } else {
                // TODO: реакционный огонь?
                let attached_unit_id = attached_unit_id.unwrap();
                active_unit_ids.insert(transporter_id); // ?
                // возможно мне не нужен сам прикрепляемый юнит, хватит и транспортера
                let attached_unit = state.unit(attached_unit_id);
                let is_attached_unit_vis = fow.is_visible(
                    db, state, attached_unit, to);
                let is_transporter_vis = fow.is_visible(
                    db, state, transporter, from);
                if is_attached_unit_vis {
                    if !is_transporter_vis {
                        events.push(CoreEvent::ShowUnit {
                            unit_info: UnitInfo {
                                pos: from,
                                attached_unit_id: None,
                                .. unit_to_info(transporter)
                            },
                        });
                    }
                    events.push(event.clone())
                } else {
                    if is_transporter_vis {
                        events.push(CoreEvent::Move {
                            unit_id: transporter_id,
                            mode: MoveMode::Fast,
                            cost: MovePoints{n: 0},
                            from: from,
                            to: to,
                        });
                        events.push(CoreEvent::HideUnit {
                            unit_id: transporter_id,
                        });
                    }
                }
            }
        },
        // TODO: ты продумал что там с реакционным огнем?
        // хотя это в simulation_step обрабатываться должно
        //
        // TODO: ты продумал что там с уничтожением буксира с прицепом?
        // TODO: ты продумал что там с уничтожением транспортера с пассажирами (высади их уже)
        CoreEvent::Detach{transporter_id, from, to} => {
            let transporter = state.unit(transporter_id);
            if transporter.player_id == player_id {
                events.push(event.clone())
            } else {
                active_unit_ids.insert(transporter_id);
                let is_from_vis = fow.is_visible(
                    db, state, transporter, from);
                let is_to_vis = fow.is_visible(
                    db, state, transporter, to);
                if is_from_vis {
                    events.push(event.clone());
                    if !is_to_vis {
                        events.push(CoreEvent::HideUnit {
                            unit_id: transporter_id,
                        });
                    }
               } else {
                    if is_to_vis {
                        events.push(CoreEvent::ShowUnit {
                            unit_info: UnitInfo {
                                pos: from,
                                attached_unit_id: None,
                                .. unit_to_info(transporter)
                            },
                        });
                        events.push(CoreEvent::Move {
                            unit_id: transporter_id,
                            mode: MoveMode::Fast,
                            cost: MovePoints{n: 0},
                            from: from,
                            to: to,
                        });
                    }
                }
            }
        },
        CoreEvent::SetReactionFireMode{unit_id, ..} => {
            let unit = state.unit(unit_id);
            if unit.player_id == player_id {
                events.push(event.clone());
            }
        },
        CoreEvent::Smoke{id, pos, unit_id} => {
            let unit_id = unit_id.expect("Core must know about everything");
            let unit = state.unit(unit_id);
            if fow.is_visible(db, state, unit, unit.pos) {
                events.push(event.clone());
            } else {
                events.push(CoreEvent::Smoke {
                    id: id,
                    pos: pos,
                    unit_id: None,
                });
            }
        },
        CoreEvent::EndTurn{..} |
        CoreEvent::RemoveSmoke{..} |
        CoreEvent::VictoryPoint{..} |
        CoreEvent::SectorOwnerChanged{..} => {
            events.push(event.clone());
        },
    }
    (events, active_unit_ids)
}
