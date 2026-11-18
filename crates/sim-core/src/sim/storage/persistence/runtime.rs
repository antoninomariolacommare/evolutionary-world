//! State whose timing/order cannot be recovered from the visible world alone.
use super::*;

#[derive(Serialize, Deserialize)]
pub(super) struct RuntimeSave {
    pub settlement_snapshots: Option<(u64, Vec<crate::sim::civ::settlements::SettlementSnapshot>)>,
    pub lineage_aggregates: HashMap<String, crate::sim::simulation::LineageAggregate>,
    pub population_limit: usize,
    pub physics: PhysicsEngine,
    pub smog: Vec<crate::sim::civ::smog::SmogSource>,
    pub pending_memory_flushes: Vec<crate::sim::simulation::PendingMemoryFlush>,
    pub lineage_crafts: HashMap<String, crate::organism::skills::Skill>,
    pub lineage_crafts_ready: bool,
    pub ferry_survey: crate::sim::tech::ferry::FerrySurvey,
    pub ferry_queues: crate::sim::tech::ferry::FerryQueues,
    pub animal_census: Vec<u32>,
    pub tribe_peril: HashMap<String, crate::sim::civ::peril::Peril>,
    pub festival_last: HashMap<String, u64>,
    pub evening_places: std::collections::BTreeMap<String, (i32, i32)>,
    pub school_places: std::collections::BTreeMap<String, (i32, i32)>,
    pub revive_cooldown: HashMap<String, u64>,
    pub teach_cooldown: HashMap<String, u64>,
    pub grave_queue: Vec<(String, f32, f32)>,
    pub cemeteries: HashMap<String, (i32, i32)>,
    pub orphans_cared: HashSet<String>,
    pub faith_empty_since: HashMap<String, u64>,
    pub active_structure_tiles: HashSet<(i32, i32)>,
    pub tile_owner: Vec<((i32, i32), String)>,
    pub trail_dirty: Vec<u32>,
    pub recent_deaths: HashMap<String, Vec<(u64, String)>>,
}

impl RuntimeSave {
    pub fn capture(sim: &Simulation) -> Self {
        Self {
            settlement_snapshots: sim
                .settlement_snapshots
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_ref()
                .map(|(tick, snapshots)| (*tick, snapshots.as_ref().clone())),
            lineage_aggregates: sim.lineage_aggregates.clone(),
            population_limit: sim.population_limit,
            physics: sim.physics.clone(),
            smog: sim.smog.clone(),
            pending_memory_flushes: sim.pending_memory_flushes.clone(),
            lineage_crafts: sim.lineage_crafts.clone(),
            lineage_crafts_ready: sim.lineage_crafts_ready,
            ferry_survey: sim.ferry_survey.clone(),
            ferry_queues: sim.ferry_queues.clone(),
            animal_census: sim.animal_census.clone(),
            tribe_peril: sim.tribe_peril.clone(),
            festival_last: sim.festival_last.clone(),
            evening_places: sim.evening_places.clone(),
            school_places: sim.school_places.clone(),
            revive_cooldown: sim.revive_cooldown.clone(),
            teach_cooldown: sim.teach_cooldown.clone(),
            grave_queue: sim.grave_queue.clone(),
            cemeteries: sim.cemeteries.clone(),
            orphans_cared: sim.orphans_cared.clone(),
            faith_empty_since: sim.faith_empty_since.clone(),
            active_structure_tiles: sim.active_structure_tiles.clone(),
            tile_owner: sim.tile_owner.iter().map(|(k, v)| (*k, v.clone())).collect(),
            trail_dirty: sim.grid.trail_dirty.iter().copied().collect(),
            recent_deaths: sim
                .recent_deaths
                .iter()
                .map(|(id, deaths)| {
                    (
                        id.clone(),
                        deaths
                            .iter()
                            .map(|(tick, cause)| (*tick, (*cause).to_owned()))
                            .collect(),
                    )
                })
                .collect(),
        }
    }

    pub fn restore(self, sim: &mut Simulation) {
        sim.settlement_snapshots = std::sync::Mutex::new(
            self.settlement_snapshots
                .map(|(tick, snapshots)| (tick, std::sync::Arc::new(snapshots))),
        );
        sim.lineage_aggregates = self.lineage_aggregates;
        sim.population_limit = self.population_limit;
        sim.physics = self.physics;
        sim.smog = self.smog;
        sim.pending_memory_flushes = self.pending_memory_flushes;
        sim.lineage_crafts = self.lineage_crafts;
        sim.lineage_crafts_ready = self.lineage_crafts_ready;
        sim.ferry_survey = self.ferry_survey;
        sim.ferry_queues = self.ferry_queues;
        sim.animal_census = self.animal_census;
        sim.tribe_peril = self.tribe_peril;
        sim.festival_last = self.festival_last;
        sim.evening_places = self.evening_places;
        sim.school_places = self.school_places;
        sim.revive_cooldown = self.revive_cooldown;
        sim.teach_cooldown = self.teach_cooldown;
        sim.grave_queue = self.grave_queue;
        sim.cemeteries = self.cemeteries;
        sim.orphans_cared = self.orphans_cared;
        sim.faith_empty_since = self.faith_empty_since;
        sim.active_structure_tiles = self.active_structure_tiles;
        sim.tile_owner = self.tile_owner.into_iter().collect();
        sim.grid.trail_dirty = self.trail_dirty.into_iter().collect();
        sim.recent_deaths = self
            .recent_deaths
            .into_iter()
            .map(|(id, deaths)| {
                (
                    id,
                    deaths
                        .into_iter()
                        .map(|(tick, cause)| (tick, death_cause(&cause).expect("validated death cause")))
                        .collect(),
                )
            })
            .collect();
    }
}

// Intern only the finite vocabulary used by note_death, without leaking input strings.
pub(super) fn death_cause(value: &str) -> Option<&'static str> {
    [
        "old_age",
        "old age",
        "starvation",
        "dehydration",
        "sickness",
        "combat",
        "murder",
        "executed",
        "war",
        "beasts",
        "drowning",
        "fire",
        "disaster",
    ]
    .into_iter()
    .find(|s| *s == value)
}
