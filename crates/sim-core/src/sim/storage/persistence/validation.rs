//! Disk/browser input boundary. Legacy migration is distinct from exact restoration.
use super::*;
use serde_json::Value;

impl Simulation {
    /// Decode a checkpoint without repairing a current-schema world. Versions 0–7
    /// are explicitly migrated; their missing historical state cannot be recovered.
    pub fn from_checkpoint_bytes(seed: u64, bytes: &[u8]) -> Result<Self, String> {
        let raw: Value = serde_json::from_slice(bytes).map_err(|e| format!("not a saved world: {e}"))?;
        // Deserialize the original bytes, NOT Value: object insertion order is part
        // of the simulation's ordered maps and Value uses sorted object keys.
        let state: SaveState =
            serde_json::from_slice(bytes).map_err(|e| format!("not a saved world: {e}"))?;
        if state.version < SAVE_SCHEMA_VERSION && state.runtime.is_some() {
            return Err("current checkpoint runtime requires an explicit current schema version".into());
        }
        if state.version > SAVE_SCHEMA_VERSION {
            return Err(format!(
                "saved by a newer build (schema v{}, this build reads up to v{})",
                state.version, SAVE_SCHEMA_VERSION
            ));
        }
        if state.version == SAVE_SCHEMA_VERSION {
            validate_current(&state)?;
            // A ChaCha word position is u128. Going through JSON bytes avoids
            // Value::Serializer rejecting valid positions larger than u64; these
            // validation trees are used only for field presence, never restoration.
            let encoded = serde_json::to_vec(&state).map_err(|e| e.to_string())?;
            let written: Value = serde_json::from_slice(&encoded).map_err(|e| e.to_string())?;
            require_written_fields(&raw, &written, "world")?;
            Ok(Self::from_save(state.world_seed, state))
        } else {
            Self::migrate_legacy_checkpoint(seed, state)
        }
    }

    /// Import an older schema using the historical repair rules. This is a new
    /// continuity baseline, not a promise to reproduce the old build's future.
    pub fn migrate_legacy_checkpoint(seed: u64, state: SaveState) -> Result<Self, String> {
        if state.version >= SAVE_SCHEMA_VERSION {
            return Err("migration accepts only checkpoint schemas 0–7".into());
        }
        let cells = state.grid.tiles.len();
        if cells != WIDTH * HEIGHT && cells != resize::OLD_WIDTH * resize::OLD_HEIGHT {
            return Err(format!("unsupported legacy map size: {cells} cells"));
        }
        tracing::warn!(from_schema = state.version, to_schema = SAVE_SCHEMA_VERSION,
            "Migrating legacy checkpoint: navigation, animal headings and temporal state were not saved; exact historical continuation is unavailable");
        // Schema 0 had no reliable seed. Schemas 1–7 wrote it, including zero.
        let world_seed = if state.version == 0 && state.world_seed == 0 {
            seed
        } else {
            state.world_seed
        };
        Ok(Self::from_save(world_seed, state))
    }
}

/// Sparse fields intentionally omitted by the writer have their documented
/// defaults. Every field the writer does emit must be present, recursively,
/// including nested structs with legacy serde(default) annotations.
fn require_written_fields(raw: &Value, written: &Value, path: &str) -> Result<(), String> {
    match (raw, written) {
        (Value::Object(input), Value::Object(output)) => {
            for (key, value) in output {
                let child = input
                    .get(key)
                    .ok_or_else(|| format!("incomplete current checkpoint: {path}.{key}"))?;
                require_written_fields(child, value, &format!("{path}.{key}"))?;
            }
        }
        (Value::Array(input), Value::Array(output)) => {
            for (i, (a, b)) in input.iter().zip(output).enumerate() {
                require_written_fields(a, b, &format!("{path}[{i}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn validate_current(state: &SaveState) -> Result<(), String> {
    let runtime = state
        .runtime
        .as_ref()
        .ok_or("incomplete current checkpoint: runtime")?;
    if state.rng.is_none() {
        return Err("incomplete current checkpoint: rng".into());
    }
    if state.sex_words.len() != 2 {
        return Err("invalid sex_words".into());
    }
    if !(crate::sim::config::MIN_POPULATION_LIMIT..=crate::sim::config::MAX_POPULATION_LIMIT)
        .contains(&runtime.population_limit)
    {
        return Err("invalid population_limit".into());
    }
    let grid = &state.grid;
    let n = WIDTH * HEIGHT;
    if [
        grid.tiles.len(),
        grid.elevation.len(),
        grid.temperature.len(),
        grid.depth.len(),
        grid.biome.len(),
        grid.fire.len(),
        grid.food_trail.len(),
        grid.water_trail.len(),
        grid.path_trail.len(),
        grid.structure.len(),
        grid.fertility.len(),
        grid.hazard.len(),
        grid.pressure.len(),
        grid.road.len(),
    ]
    .iter()
    .any(|&len| len != n)
        || grid.volcanoes.is_none()
        || grid.pool_centers.is_none()
    {
        return Err(format!(
            "incomplete current checkpoint: grid layers (map has {} cells)",
            grid.tiles.len()
        ));
    }
    if runtime.trail_dirty.iter().any(|&i| i as usize >= n) {
        return Err("invalid trail index".into());
    }
    for deaths in runtime.recent_deaths.values() {
        if deaths
            .iter()
            .any(|(_, cause)| super::runtime::death_cause(cause).is_none())
        {
            return Err("invalid recent death cause".into());
        }
    }
    let mut ids = HashSet::default();
    for org in &state.organisms {
        if !ids.insert(&org.id) {
            return Err("duplicate organism id".into());
        }
        let saved = org
            .runtime
            .as_ref()
            .ok_or("incomplete current checkpoint: organism runtime")?;
        if (!saved.route.steps.is_empty() && saved.route.next > saved.route.steps.len())
            || saved
                .route
                .other
                .as_ref()
                .is_some_and(|(_, steps, next)| !steps.is_empty() && *next > steps.len())
        {
            return Err("invalid navigation cursor".into());
        }
    }
    for animal in &state.animals {
        if animal.runtime.is_none() || animal.heading.is_none_or(|heading| heading >= 8) {
            return Err("incomplete or invalid animal heading".into());
        }
    }
    Ok(())
}
