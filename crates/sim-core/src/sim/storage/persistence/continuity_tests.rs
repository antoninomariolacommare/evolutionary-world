//! Regression contract: bytes cover authoritative state, including ordered maps.
use super::*;

fn checkpoint(sim: &Simulation) -> Vec<u8> {
    serde_json::to_vec(&sim.to_save_state()).unwrap()
}

fn assert_same(a: &Simulation, b: &Simulation) {
    let left = checkpoint(a);
    let right = checkpoint(b);
    if left != right {
        if let Ok(dir) = std::env::var("CHECKPOINT_DIFF_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            for (name, bytes) in [("continuous", &left), ("restored", &right)] {
                std::fs::write(
                    format!("{dir}/seed-{}-tick-{}-{name}.json", a.world_seed, a.tick_count),
                    bytes,
                )
                .unwrap();
            }
        }
        let offset = left
            .iter()
            .zip(&right)
            .position(|(a, b)| a != b)
            .unwrap_or(left.len().min(right.len()));
        let start = offset.saturating_sub(80);
        panic!(
            "checkpoint differs at tick {}, byte {offset}:\ncontinuous: {}\nrestored: {}",
            a.tick_count,
            String::from_utf8_lossy(&left[start..(offset + 160).min(left.len())]),
            String::from_utf8_lossy(&right[start..(offset + 160).min(right.len())])
        );
    }
    assert_eq!(
        serde_json::to_string(&a.rng).unwrap(),
        serde_json::to_string(&b.rng).unwrap()
    );
}

#[test]
fn current_checkpoint_seed_42_continues_exactly_at_6001_and_6100() {
    let mut continuous = Simulation::new(42);
    continuous.tick_n(6000);
    let mut resumed = Simulation::from_checkpoint_bytes(999, &checkpoint(&continuous)).unwrap();
    assert_same(&continuous, &resumed); // zero-tick round trip
    continuous.tick();
    resumed.tick();
    assert_same(&continuous, &resumed);
    continuous.tick_n(99);
    resumed.tick_n(99);
    assert_same(&continuous, &resumed);
}

#[test]
fn current_checkpoints_continue_across_cadences_for_multiple_seeds() {
    for seed in [0, 1, 7] {
        let mut continuous = Simulation::new(seed);
        continuous.set_population_limit(777);
        continuous.tick_n(599);
        let mut resumed = Simulation::from_checkpoint_bytes(42, &checkpoint(&continuous)).unwrap();
        assert_eq!(resumed.world_seed, seed);
        assert_eq!(resumed.population_limit(), 777);
        assert_same(&continuous, &resumed);
        for step in 1..=205 {
            continuous.tick();
            resumed.tick();
            if [1, 6, 101, 205].contains(&step) {
                assert_same(&continuous, &resumed);
            }
        }
        assert_same(&continuous, &resumed);
    }
}

#[test]
fn current_checkpoint_preserves_empty_memories_navigation_work_and_temporal_state() {
    use crate::organism::{organism::Harm, skills::Skill};
    let mut sim = Simulation::new(42);
    sim.tick_n(123);
    let id = sim.organisms[0].id.clone();
    let lineage = sim.organisms[0].lineage_id.clone();
    let org = &mut sim.organisms[0];
    org.vocabulary = Default::default();
    org.memories = Default::default();
    org.home_x = 0.0;
    org.home_y = 0.0;
    org.last_harm = Some((Harm::Fire, 122));
    org.skills.set(Skill::Foraging, 0.1234567);
    org.skills.set_feeling(Skill::Foraging, 0.0001234);
    org.vx_smooth = 0.12345;
    org.vy_smooth = -0.54321;
    org.work = Some(crate::sim::agents::daily_work::WorkPlan {
        skill: Skill::Foraging,
        spot: (30, 30),
        until: 220,
        harvest: false,
        misses: 2,
    });
    org.route.get_mut().mark_blocked((32, 33));
    org.org_trust.insert("a-remembered-person".into(), 0.375);
    sim.animals[0].heading = 7;
    sim.animals[0].sleeping = true;
    sim.animals[0].away = true;
    sim.animals[0].wary = 0.234567;
    sim.animals[0].hunting = true;
    sim.buildings.push(crate::sim::buildings::Building::new(
        9000,
        crate::sim::buildings::BuildingKind::Hut,
        40,
        40,
        Some(lineage.clone()),
        100,
    ));
    sim.buildings.last_mut().unwrap().last_work_tick = Some(122);
    sim.next_building_id = 9001;
    sim.next_battle_id = 1;
    sim.festival_last.insert(lineage.clone(), 120);
    sim.teach_cooldown.insert(lineage.clone(), 119);
    sim.revive_cooldown.insert(lineage.clone(), 118);
    sim.faith_empty_since.insert("rel-extinct".into(), 110);
    sim.orphans_cared.insert(id);
    sim.grave_queue.push((lineage.clone(), 33.0, 34.0));
    sim.recent_deaths
        .entry(lineage.clone())
        .or_default()
        .push_back((120, "old_age"));
    // The most recent claimant cannot be inferred from the forward map when claims overlap.
    sim.territory.entry(lineage.clone()).or_default().insert((30, 30));
    sim.territory
        .entry("another-clan".into())
        .or_default()
        .insert((30, 30));
    sim.tile_owner.insert((30, 30), lineage);
    // A valid current checkpoint must not normalize the ocean on import.
    sim.grid.fertility[0] = 0.0036;
    let mut resumed = Simulation::from_checkpoint_bytes(9, &checkpoint(&sim)).unwrap();
    assert_same(&sim, &resumed);
    for _ in 0..30 {
        sim.tick();
        resumed.tick();
        assert_same(&sim, &resumed);
    }
    assert_same(&sim, &resumed);
}

#[test]
fn current_checkpoint_rejects_missing_fields_including_nested_defaults() {
    let sim = Simulation::new(42);
    let original: serde_json::Value = serde_json::from_slice(&checkpoint(&sim)).unwrap();
    for path in [
        vec!["version"],
        vec!["rng"],
        vec!["rng", "word_pos"],
        vec!["runtime"],
        vec!["world_seed"],
        vec!["tick_count"],
        vec!["grid", "fertility"],
        vec!["weather", "wind_x"],
        vec!["runtime", "population_limit"],
        vec!["runtime", "physics"],
    ] {
        let mut raw = original.clone();
        let mut parent = &mut raw;
        for key in &path[..path.len() - 1] {
            parent = &mut parent[*key];
        }
        parent.as_object_mut().unwrap().remove(*path.last().unwrap());
        assert!(
            Simulation::from_checkpoint_bytes(42, &serde_json::to_vec(&raw).unwrap()).is_err(),
            "accepted missing {path:?}"
        );
    }
    for field in ["runtime", "traits", "q_table", "last_reproduced"] {
        let mut raw = original.clone();
        raw["organisms"][0].as_object_mut().unwrap().remove(field);
        assert!(
            Simulation::from_checkpoint_bytes(42, &serde_json::to_vec(&raw).unwrap()).is_err(),
            "accepted missing organism {field}"
        );
    }
    let mut raw = original.clone();
    raw["rng"] = serde_json::Value::Null;
    assert!(Simulation::from_checkpoint_bytes(42, &serde_json::to_vec(&raw).unwrap()).is_err());
    let mut raw = original;
    raw["animals"][0].as_object_mut().unwrap().remove("heading");
    assert!(Simulation::from_checkpoint_bytes(42, &serde_json::to_vec(&raw).unwrap()).is_err());
}

#[test]
fn legacy_migration_is_repeatable_but_current_schema_never_uses_repairs() {
    let sim = Simulation::new(0);
    let mut raw: serde_json::Value = serde_json::from_slice(&checkpoint(&sim)).unwrap();
    raw["version"] = 7.into();
    raw.as_object_mut().unwrap().remove("runtime");
    for org in raw["organisms"].as_array_mut().unwrap() {
        org.as_object_mut().unwrap().remove("runtime");
    }
    for animal in raw["animals"].as_array_mut().unwrap() {
        animal.as_object_mut().unwrap().remove("heading");
        animal.as_object_mut().unwrap().remove("runtime");
    }
    let bytes = serde_json::to_vec(&raw).unwrap();
    let mut a = Simulation::from_checkpoint_bytes(42, &bytes).unwrap();
    let mut b = Simulation::from_checkpoint_bytes(99, &bytes).unwrap();
    assert_eq!(a.world_seed, 0);
    assert_same(&a, &b);
    let mut resumed = Simulation::from_checkpoint_bytes(42, &checkpoint(&a)).unwrap();
    a.tick_n(40);
    b.tick_n(40);
    resumed.tick_n(40);
    assert_same(&a, &b);
    assert_same(&a, &resumed);
    assert!(Simulation::migrate_legacy_checkpoint(42, sim.to_save_state()).is_err());
}

#[test]
fn rng_stream_and_large_word_position_resume_without_reseeding() {
    use rand::RngExt;
    let mut a = Simulation::new(0);
    a.rng.set_stream(u64::MAX - 17);
    a.rng.set_word_pos((1u128 << 65) + 7);
    let mut b = Simulation::from_checkpoint_bytes(42, &checkpoint(&a)).unwrap();
    assert_same(&a, &b);
    // Cross several ChaCha buffer refills, starting at an unaligned position.
    for _ in 0..257 {
        assert_eq!(a.rng.random::<u64>(), b.rng.random::<u64>());
    }
    assert_same(&a, &b);
}
