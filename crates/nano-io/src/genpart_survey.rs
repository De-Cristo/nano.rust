use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use serde::Serialize;

pub const KEY_PDGS: [i32; 10] = [25, 22, 113, 211, -211, 111, 221, 223, 213, -213];
pub const INTEREST_PDGS: [i32; 5] = [25, 22, 113, 211, -211];

#[derive(Clone, Debug, PartialEq)]
pub struct GenPartRecord {
    pub pdg_id: i32,
    pub mother: Option<usize>,
    pub status: i32,
    pub status_flags: u16,
    pub pt: f64,
    pub eta: f64,
    pub phi: f64,
    pub mass: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EventSurveySummary {
    pub n_genpart: usize,
    pub contains_higgs: bool,
    pub contains_rho0: bool,
    pub contains_photon: bool,
    pub contains_pi_plus_pi_minus: bool,
    pub contains_h_gamma_pions: bool,
    pub explicit_h_to_rho_gamma: bool,
    pub explicit_rho_to_pions: bool,
    pub any_photon_with_higgs_ancestor: bool,
    pub any_pi_plus_with_higgs_ancestor: bool,
    pub any_pi_minus_with_higgs_ancestor: bool,
    pub any_pi_plus_with_rho_ancestor: bool,
    pub any_pi_minus_with_rho_ancestor: bool,
    pub any_rho_with_higgs_ancestor: bool,
    pub signed_pdg_counts: BTreeMap<i32, u64>,
    pub abs_pdg_counts: BTreeMap<i32, u64>,
    pub status_counts: BTreeMap<(i32, i32), u64>,
    pub status_flags_counts: BTreeMap<(i32, u16), u64>,
    pub status_status_flags_counts: BTreeMap<(i32, i32, u16), u64>,
    pub mother_daughter_counts: BTreeMap<(i32, i32), u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurveyedEvent {
    pub event_id: String,
    pub particles: Vec<GenPartRecord>,
    pub summary: EventSurveySummary,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GenPartSurvey {
    pub processed_events: u64,
    pub events_with_genpart: u64,
    pub events_with_higgs: u64,
    pub events_with_rho0: u64,
    pub events_with_photon: u64,
    pub events_with_pi_plus_pi_minus: u64,
    pub events_with_h_gamma_pions: u64,
    pub events_with_explicit_h_to_rho_gamma: u64,
    pub events_with_explicit_rho_to_pions: u64,
    pub events_with_photon_higgs_ancestor: u64,
    pub events_with_pi_plus_higgs_ancestor: u64,
    pub events_with_pi_minus_higgs_ancestor: u64,
    pub events_with_pi_plus_rho_ancestor: u64,
    pub events_with_pi_minus_rho_ancestor: u64,
    pub events_with_rho_higgs_ancestor: u64,
    pub signed_pdg_counts: BTreeMap<i32, u64>,
    pub abs_pdg_counts: BTreeMap<i32, u64>,
    pub status_counts: BTreeMap<(i32, i32), u64>,
    pub status_flags_counts: BTreeMap<(i32, u16), u64>,
    pub status_status_flags_counts: BTreeMap<(i32, i32, u16), u64>,
    pub mother_daughter_counts: BTreeMap<(i32, i32), u64>,
    pub events: Vec<SurveyedEvent>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InterestingExamples {
    pub with_higgs: Vec<SurveyedEvent>,
    pub with_rho0: Vec<SurveyedEvent>,
    pub with_gamma_pions_no_rho: Vec<SurveyedEvent>,
    pub current_truth_not_found: Vec<SurveyedEvent>,
}

impl InterestingExamples {
    pub fn from_events(events: &[SurveyedEvent], limit: usize) -> Self {
        let mut examples = Self::default();
        for event in events {
            if event.summary.contains_higgs && examples.with_higgs.len() < limit {
                examples.with_higgs.push(event.clone());
            }
            if event.summary.contains_rho0 && examples.with_rho0.len() < limit {
                examples.with_rho0.push(event.clone());
            }
            if event.summary.contains_photon
                && event.summary.contains_pi_plus_pi_minus
                && !event.summary.contains_rho0
                && examples.with_gamma_pions_no_rho.len() < limit
            {
                examples.with_gamma_pions_no_rho.push(event.clone());
            }
            if !event.summary.explicit_h_to_rho_gamma
                && !event.summary.contains_h_gamma_pions
                && examples.current_truth_not_found.len() < limit
            {
                examples.current_truth_not_found.push(event.clone());
            }
        }
        examples
    }
}

pub fn has_ancestor_pdg_id(
    particles: &[GenPartRecord],
    index: usize,
    ancestor_pdg_id: i32,
) -> bool {
    let Some(mut current) = particles.get(index).and_then(|particle| particle.mother) else {
        return false;
    };
    let mut visited = BTreeSet::new();
    while visited.insert(current) {
        let Some(particle) = particles.get(current) else {
            return false;
        };
        if particle.pdg_id == ancestor_pdg_id {
            return true;
        }
        let Some(next) = particle.mother else {
            return false;
        };
        current = next;
    }
    false
}

pub fn survey_event(particles: &[GenPartRecord]) -> EventSurveySummary {
    let mut summary = EventSurveySummary {
        n_genpart: particles.len(),
        ..EventSurveySummary::default()
    };
    let mut mother_children: BTreeMap<usize, Vec<i32>> = BTreeMap::new();

    for (index, particle) in particles.iter().enumerate() {
        summary.contains_higgs |= particle.pdg_id == 25;
        summary.contains_rho0 |= particle.pdg_id == 113;
        summary.contains_photon |= particle.pdg_id == 22;
        *summary
            .signed_pdg_counts
            .entry(particle.pdg_id)
            .or_default() += 1;
        *summary
            .abs_pdg_counts
            .entry(particle.pdg_id.abs())
            .or_default() += 1;
        *summary
            .status_counts
            .entry((particle.pdg_id, particle.status))
            .or_default() += 1;
        *summary
            .status_flags_counts
            .entry((particle.pdg_id, particle.status_flags))
            .or_default() += 1;
        *summary
            .status_status_flags_counts
            .entry((particle.pdg_id, particle.status, particle.status_flags))
            .or_default() += 1;

        let mother_pdg = particle
            .mother
            .and_then(|mother| particles.get(mother))
            .map_or(0, |mother| mother.pdg_id);
        *summary
            .mother_daughter_counts
            .entry((mother_pdg, particle.pdg_id))
            .or_default() += 1;
        if let Some(mother) = particle.mother.filter(|mother| *mother < particles.len()) {
            mother_children
                .entry(mother)
                .or_default()
                .push(particle.pdg_id);
        }
        if particle.pdg_id == 22 && has_ancestor_pdg_id(particles, index, 25) {
            summary.any_photon_with_higgs_ancestor = true;
        }
        if particle.pdg_id == 211 && has_ancestor_pdg_id(particles, index, 25) {
            summary.any_pi_plus_with_higgs_ancestor = true;
        }
        if particle.pdg_id == -211 && has_ancestor_pdg_id(particles, index, 25) {
            summary.any_pi_minus_with_higgs_ancestor = true;
        }
        if particle.pdg_id == 211 && has_ancestor_pdg_id(particles, index, 113) {
            summary.any_pi_plus_with_rho_ancestor = true;
        }
        if particle.pdg_id == -211 && has_ancestor_pdg_id(particles, index, 113) {
            summary.any_pi_minus_with_rho_ancestor = true;
        }
        if particle.pdg_id == 113 && has_ancestor_pdg_id(particles, index, 25) {
            summary.any_rho_with_higgs_ancestor = true;
        }
    }

    let has_pi_plus = summary.signed_pdg_counts.contains_key(&211);
    let has_pi_minus = summary.signed_pdg_counts.contains_key(&-211);
    summary.contains_pi_plus_pi_minus = has_pi_plus && has_pi_minus;
    summary.contains_h_gamma_pions =
        summary.contains_higgs && summary.contains_photon && summary.contains_pi_plus_pi_minus;

    for (index, particle) in particles.iter().enumerate() {
        let daughters = mother_children
            .get(&index)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if particle.pdg_id == 25 && daughters.contains(&113) && daughters.contains(&22) {
            summary.explicit_h_to_rho_gamma = true;
        }
        if particle.pdg_id == 113 && daughters.contains(&211) && daughters.contains(&-211) {
            summary.explicit_rho_to_pions = true;
        }
    }

    summary
}

pub fn survey_events(
    events: impl IntoIterator<Item = (String, Vec<GenPartRecord>)>,
    example_limit: usize,
) -> GenPartSurvey {
    let mut survey = GenPartSurvey::default();
    for (event_id, particles) in events {
        let summary = survey_event(&particles);
        survey.processed_events += 1;
        survey.events_with_genpart += u64::from(summary.n_genpart > 0);
        survey.events_with_higgs += u64::from(summary.contains_higgs);
        survey.events_with_rho0 += u64::from(summary.contains_rho0);
        survey.events_with_photon += u64::from(summary.contains_photon);
        survey.events_with_pi_plus_pi_minus += u64::from(summary.contains_pi_plus_pi_minus);
        survey.events_with_h_gamma_pions += u64::from(summary.contains_h_gamma_pions);
        survey.events_with_explicit_h_to_rho_gamma += u64::from(summary.explicit_h_to_rho_gamma);
        survey.events_with_explicit_rho_to_pions += u64::from(summary.explicit_rho_to_pions);
        survey.events_with_photon_higgs_ancestor +=
            u64::from(summary.any_photon_with_higgs_ancestor);
        survey.events_with_pi_plus_higgs_ancestor +=
            u64::from(summary.any_pi_plus_with_higgs_ancestor);
        survey.events_with_pi_minus_higgs_ancestor +=
            u64::from(summary.any_pi_minus_with_higgs_ancestor);
        survey.events_with_pi_plus_rho_ancestor += u64::from(summary.any_pi_plus_with_rho_ancestor);
        survey.events_with_pi_minus_rho_ancestor +=
            u64::from(summary.any_pi_minus_with_rho_ancestor);
        survey.events_with_rho_higgs_ancestor += u64::from(summary.any_rho_with_higgs_ancestor);
        merge_map(&mut survey.signed_pdg_counts, &summary.signed_pdg_counts);
        merge_map(&mut survey.abs_pdg_counts, &summary.abs_pdg_counts);
        merge_map(&mut survey.status_counts, &summary.status_counts);
        merge_map(
            &mut survey.status_flags_counts,
            &summary.status_flags_counts,
        );
        merge_map(
            &mut survey.status_status_flags_counts,
            &summary.status_status_flags_counts,
        );
        merge_map(
            &mut survey.mother_daughter_counts,
            &summary.mother_daughter_counts,
        );
        if survey.events.len() < example_limit.saturating_mul(8).max(1) {
            survey.events.push(SurveyedEvent {
                event_id,
                particles,
                summary,
            });
        }
    }
    survey
}

fn merge_map<K: Ord + Copy>(target: &mut BTreeMap<K, u64>, source: &BTreeMap<K, u64>) {
    for (key, count) in source {
        *target.entry(*key).or_default() += count;
    }
}

#[derive(Serialize)]
pub struct JsonSurveySummary {
    pub processed_events: u64,
    pub events_with_genpart: u64,
    pub events_with_higgs: u64,
    pub events_with_rho0: u64,
    pub events_with_photon: u64,
    pub events_with_pi_plus_pi_minus: u64,
    pub events_with_h_gamma_pions: u64,
    pub events_with_explicit_h_to_rho_gamma: u64,
    pub events_with_explicit_rho_to_pions: u64,
    pub events_with_photon_higgs_ancestor: u64,
    pub events_with_pi_plus_higgs_ancestor: u64,
    pub events_with_pi_minus_higgs_ancestor: u64,
    pub events_with_pi_plus_rho_ancestor: u64,
    pub events_with_pi_minus_rho_ancestor: u64,
    pub events_with_rho_higgs_ancestor: u64,
    pub signed_pdg_counts: BTreeMap<String, u64>,
    pub abs_pdg_counts: BTreeMap<String, u64>,
    pub status_counts: BTreeMap<String, u64>,
    pub status_flags_counts: BTreeMap<String, u64>,
    pub status_status_flags_counts: BTreeMap<String, u64>,
    pub mother_daughter_counts: BTreeMap<String, u64>,
    pub recommendation: String,
}

impl From<&GenPartSurvey> for JsonSurveySummary {
    fn from(survey: &GenPartSurvey) -> Self {
        Self {
            processed_events: survey.processed_events,
            events_with_genpart: survey.events_with_genpart,
            events_with_higgs: survey.events_with_higgs,
            events_with_rho0: survey.events_with_rho0,
            events_with_photon: survey.events_with_photon,
            events_with_pi_plus_pi_minus: survey.events_with_pi_plus_pi_minus,
            events_with_h_gamma_pions: survey.events_with_h_gamma_pions,
            events_with_explicit_h_to_rho_gamma: survey.events_with_explicit_h_to_rho_gamma,
            events_with_explicit_rho_to_pions: survey.events_with_explicit_rho_to_pions,
            events_with_photon_higgs_ancestor: survey.events_with_photon_higgs_ancestor,
            events_with_pi_plus_higgs_ancestor: survey.events_with_pi_plus_higgs_ancestor,
            events_with_pi_minus_higgs_ancestor: survey.events_with_pi_minus_higgs_ancestor,
            events_with_pi_plus_rho_ancestor: survey.events_with_pi_plus_rho_ancestor,
            events_with_pi_minus_rho_ancestor: survey.events_with_pi_minus_rho_ancestor,
            events_with_rho_higgs_ancestor: survey.events_with_rho_higgs_ancestor,
            signed_pdg_counts: map_i32(&survey.signed_pdg_counts),
            abs_pdg_counts: map_i32(&survey.abs_pdg_counts),
            status_counts: map_pair_i32(&survey.status_counts, ":"),
            status_flags_counts: map_pair_i32_u16(&survey.status_flags_counts, ":"),
            status_status_flags_counts: map_triple(&survey.status_status_flags_counts),
            mother_daughter_counts: map_pair_i32(&survey.mother_daughter_counts, "->"),
            recommendation: recommendation(survey).to_string(),
        }
    }
}

pub fn recommendation(survey: &GenPartSurvey) -> &'static str {
    if survey.events_with_explicit_h_to_rho_gamma > 0
        && survey.events_with_explicit_rho_to_pions > 0
    {
        "explicit rho0 exists; use last-copy rho0 and descendants"
    } else if survey.events_with_photon_higgs_ancestor > 0
        && survey.events_with_pi_plus_higgs_ancestor > 0
        && survey.events_with_pi_minus_higgs_ancestor > 0
    {
        "rho0 absent or incomplete; match final-state photon and charged pions from Higgs ancestry"
    } else {
        "ancestry unreliable; use nearest final-state photon/pi+pi- matching in signal MC, plus generator-level invariant-mass checks"
    }
}

pub fn text_summary(survey: &GenPartSurvey) -> String {
    let mut out = String::new();
    writeln!(out, "GenPart topology survey").unwrap();
    writeln!(out, "processed_events: {}", survey.processed_events).unwrap();
    writeln!(out, "events_with_genpart: {}", survey.events_with_genpart).unwrap();
    writeln!(out, "events_with_higgs: {}", survey.events_with_higgs).unwrap();
    writeln!(out, "events_with_rho0: {}", survey.events_with_rho0).unwrap();
    writeln!(out, "events_with_photon: {}", survey.events_with_photon).unwrap();
    writeln!(
        out,
        "events_with_pi_plus_pi_minus: {}",
        survey.events_with_pi_plus_pi_minus
    )
    .unwrap();
    writeln!(
        out,
        "events_with_h_gamma_pions: {}",
        survey.events_with_h_gamma_pions
    )
    .unwrap();
    writeln!(
        out,
        "events_with_explicit_h_to_rho_gamma: {}",
        survey.events_with_explicit_h_to_rho_gamma
    )
    .unwrap();
    writeln!(
        out,
        "events_with_explicit_rho_to_pions: {}",
        survey.events_with_explicit_rho_to_pions
    )
    .unwrap();
    writeln!(
        out,
        "events_with_photon_higgs_ancestor: {}",
        survey.events_with_photon_higgs_ancestor
    )
    .unwrap();
    writeln!(
        out,
        "events_with_pi_plus_higgs_ancestor: {}",
        survey.events_with_pi_plus_higgs_ancestor
    )
    .unwrap();
    writeln!(
        out,
        "events_with_pi_minus_higgs_ancestor: {}",
        survey.events_with_pi_minus_higgs_ancestor
    )
    .unwrap();
    writeln!(
        out,
        "events_with_pi_plus_rho_ancestor: {}",
        survey.events_with_pi_plus_rho_ancestor
    )
    .unwrap();
    writeln!(
        out,
        "events_with_pi_minus_rho_ancestor: {}",
        survey.events_with_pi_minus_rho_ancestor
    )
    .unwrap();
    writeln!(
        out,
        "events_with_rho_higgs_ancestor: {}",
        survey.events_with_rho_higgs_ancestor
    )
    .unwrap();
    writeln!(out, "recommendation: {}", recommendation(survey)).unwrap();
    write_count_table(
        &mut out,
        "key signed pdgId counts",
        &survey.signed_pdg_counts,
        KEY_PDGS,
    );
    write_count_table(
        &mut out,
        "key abs pdgId counts",
        &survey.abs_pdg_counts,
        KEY_PDGS,
    );
    writeln!(out, "top abs pdgId counts:").unwrap();
    for (pdg, count) in top_counts(&survey.abs_pdg_counts, 20) {
        writeln!(out, "  {pdg}: {count}").unwrap();
    }
    write_relation_table(
        &mut out,
        "mother -> daughter counts",
        &survey.mother_daughter_counts,
    );
    write_status_table(&mut out, "status counts", &survey.status_counts);
    write_flags_table(&mut out, "statusFlags counts", &survey.status_flags_counts);
    write_status_flags_table(
        &mut out,
        "status + statusFlags counts",
        &survey.status_status_flags_counts,
    );
    out
}

pub fn examples_text(survey: &GenPartSurvey, limit: usize) -> String {
    let examples = InterestingExamples::from_events(&survey.events, limit);
    let mut out = String::new();
    write_example_group(&mut out, "event with pdgId 25", &examples.with_higgs);
    write_example_group(&mut out, "event with pdgId 113", &examples.with_rho0);
    write_example_group(
        &mut out,
        "event with 22 and pi+pi- but no rho0",
        &examples.with_gamma_pions_no_rho,
    );
    write_example_group(
        &mut out,
        "event where simple truth finder would say not_found",
        &examples.current_truth_not_found,
    );
    out
}

fn write_example_group(out: &mut String, title: &str, events: &[SurveyedEvent]) {
    writeln!(out, "## {title}").unwrap();
    if events.is_empty() {
        writeln!(out, "none").unwrap();
        return;
    }
    for event in events {
        writeln!(out, "event: {}", event.event_id).unwrap();
        writeln!(
            out,
            "index pdgId status statusFlags mother_index mother_pdgId pt eta phi mass"
        )
        .unwrap();
        for (index, particle) in event.particles.iter().enumerate() {
            let mother_pdg = particle
                .mother
                .and_then(|mother| event.particles.get(mother))
                .map_or(0, |mother| mother.pdg_id);
            if particle_of_interest(event, index) {
                writeln!(
                    out,
                    "{index} {} {} {} {} {} {:.6} {:.6} {:.6} {:.6}",
                    particle.pdg_id,
                    particle.status,
                    particle.status_flags,
                    particle
                        .mother
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "-1".to_string()),
                    mother_pdg,
                    particle.pt,
                    particle.eta,
                    particle.phi,
                    particle.mass
                )
                .unwrap();
            }
        }
    }
}

fn particle_of_interest(event: &SurveyedEvent, index: usize) -> bool {
    let particle = &event.particles[index];
    INTEREST_PDGS.contains(&particle.pdg_id)
        || particle
            .mother
            .and_then(|mother| event.particles.get(mother))
            .is_some_and(|mother| INTEREST_PDGS.contains(&mother.pdg_id))
}

fn write_count_table(out: &mut String, title: &str, counts: &BTreeMap<i32, u64>, keys: [i32; 10]) {
    writeln!(out, "{title}:").unwrap();
    for key in keys {
        writeln!(out, "  {key}: {}", counts.get(&key).copied().unwrap_or(0)).unwrap();
    }
}

fn write_relation_table(out: &mut String, title: &str, counts: &BTreeMap<(i32, i32), u64>) {
    writeln!(out, "{title}:").unwrap();
    for ((mother, daughter), count) in counts {
        if INTEREST_PDGS.contains(mother) || INTEREST_PDGS.contains(daughter) {
            writeln!(out, "  {mother}->{daughter}: {count}").unwrap();
        }
    }
}

fn write_status_table(out: &mut String, title: &str, counts: &BTreeMap<(i32, i32), u64>) {
    writeln!(out, "{title}:").unwrap();
    for ((pdg, status), count) in counts {
        if INTEREST_PDGS.contains(pdg) {
            writeln!(out, "  {pdg}:{status}: {count}").unwrap();
        }
    }
}

fn write_flags_table(out: &mut String, title: &str, counts: &BTreeMap<(i32, u16), u64>) {
    writeln!(out, "{title}:").unwrap();
    for ((pdg, flags), count) in counts {
        if INTEREST_PDGS.contains(pdg) {
            writeln!(out, "  {pdg}:{flags}: {count}").unwrap();
        }
    }
}

fn write_status_flags_table(
    out: &mut String,
    title: &str,
    counts: &BTreeMap<(i32, i32, u16), u64>,
) {
    writeln!(out, "{title}:").unwrap();
    for ((pdg, status, flags), count) in counts {
        if INTEREST_PDGS.contains(pdg) {
            writeln!(out, "  {pdg}:{status}:{flags}: {count}").unwrap();
        }
    }
}

fn top_counts(counts: &BTreeMap<i32, u64>, limit: usize) -> Vec<(i32, u64)> {
    let mut values = counts
        .iter()
        .map(|(pdg, count)| (*pdg, *count))
        .collect::<Vec<_>>();
    values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    values.truncate(limit);
    values
}

fn map_i32(source: &BTreeMap<i32, u64>) -> BTreeMap<String, u64> {
    source
        .iter()
        .map(|(key, value)| (key.to_string(), *value))
        .collect()
}

fn map_pair_i32(source: &BTreeMap<(i32, i32), u64>, sep: &str) -> BTreeMap<String, u64> {
    source
        .iter()
        .map(|((left, right), value)| (format!("{left}{sep}{right}"), *value))
        .collect()
}

fn map_pair_i32_u16(source: &BTreeMap<(i32, u16), u64>, sep: &str) -> BTreeMap<String, u64> {
    source
        .iter()
        .map(|((left, right), value)| (format!("{left}{sep}{right}"), *value))
        .collect()
}

fn map_triple(source: &BTreeMap<(i32, i32, u16), u64>) -> BTreeMap<String, u64> {
    source
        .iter()
        .map(|((pdg, status, flags), value)| (format!("{pdg}:{status}:{flags}"), *value))
        .collect()
}
