use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::config::EvalsConfig;
use crate::domain::eval::{
    Difficulty, EvalMetadata, EvalQueryType, EvalRecord, GroundTruthFact, RagasRecord,
};
use crate::domain::patient::{Gender, PatientRecord, Severity};

/// Drugs the reaction model can produce reactions for (see `generation::simulate_reactions`).
const REACTION_DRUGS: &[&str] = &["DrugX", "DrugY"];

/// Every drug the medication model can prescribe.
const ALL_DRUGS: &[&str] = &[
    "DrugX",
    "DrugY",
    "DrugZ",
    "Aspirin",
    "Metformin",
    "Lisinopril",
    "Albuterol",
];

/// Every comorbidity the condition model can assign.
const CONDITIONS: &[&str] = &[
    "diabetes",
    "hypertension",
    "asthma",
    "chronic_kidney_disease",
    "coronary_artery_disease",
    "copd",
    "obesity",
];

/// Drugs and conditions that never appear in the corpus. Negative controls probe these,
/// but the generator still verifies emptiness against the data before asserting "None".
const UNKNOWN_DRUGS: &[&str] = &["DrugQ", "DrugW", "Cephalexin", "Warfarin"];
const UNKNOWN_CONDITIONS: &[&str] = &["lupus", "psoriasis", "cirrhosis", "epilepsy"];

/// Context cap for queries whose answer is an aggregate rather than a patient list.
const AGGREGATE_CONTEXT_LIMIT: usize = 5;

pub struct EvalContext {
    pub records: Vec<PatientRecord>,
}

impl EvalContext {
    pub fn find_by_id(&self, id: &str) -> Option<&PatientRecord> {
        self.records.iter().find(|r| r.patient_id == id)
    }

    pub fn exposed_to(&self, med: &str) -> Vec<&PatientRecord> {
        self.records
            .iter()
            .filter(|r| r.medications.iter().any(|m| m == med))
            .collect()
    }

    pub fn reactors_to(&self, med: &str) -> Vec<&PatientRecord> {
        self.records
            .iter()
            .filter(|r| r.allergic_reaction && r.reaction_medication.as_deref() == Some(med))
            .collect()
    }

    pub fn severe_reactors_to(&self, med: &str) -> Vec<&PatientRecord> {
        self.reactors_to(med)
            .into_iter()
            .filter(|r| matches!(r.reaction_severity, Some(Severity::Severe)))
            .collect()
    }

    pub fn reactors_with_type(&self, med: &str, reaction: &str) -> Vec<&PatientRecord> {
        self.reactors_to(med)
            .into_iter()
            .filter(|r| r.reaction_type.as_deref() == Some(reaction))
            .collect()
    }

    pub fn with_condition(&self, condition: &str) -> Vec<&PatientRecord> {
        self.records
            .iter()
            .filter(|r| r.comorbidities.iter().any(|c| c == condition))
            .collect()
    }

    /// Distinct reaction types present in the corpus, sorted for determinism.
    pub fn reaction_types(&self) -> Vec<String> {
        let mut types: Vec<String> = self
            .records
            .iter()
            .filter_map(|r| r.reaction_type.clone())
            .collect();
        types.sort();
        types.dedup();
        types
    }

    /// Distinct regions present in the corpus, sorted for determinism.
    pub fn regions(&self) -> Vec<String> {
        let mut regions: Vec<String> = self.records.iter().map(|r| r.region.clone()).collect();
        regions.sort();
        regions.dedup();
        regions
    }
}

/// A generated query plus the records that justify its answer. `build` turns this into an
/// `EvalRecord`, deriving contexts and chunk ids from `matches` so ground truth and
/// retrieval targets can never drift apart.
struct EvalDraft<'a> {
    query: String,
    answer: String,
    matches: Vec<&'a PatientRecord>,
    facts: Vec<GroundTruthFact>,
    query_type: EvalQueryType,
    difficulty: Difficulty,
    /// `None` includes every match's notes; `Some(n)` caps them (for aggregate answers).
    context_limit: Option<usize>,
}

impl<'a> EvalDraft<'a> {
    fn build(self, idx: u64) -> EvalRecord {
        let limit = self.context_limit.unwrap_or(self.matches.len());
        let cited = self.matches.iter().take(limit);

        let contexts: Vec<String> = cited
            .clone()
            .flat_map(|r| r.clinical_notes.iter().map(|n| n.text.clone()))
            .collect();
        let context_chunk_ids: Vec<String> = cited
            .flat_map(|r| {
                r.clinical_notes
                    .iter()
                    .enumerate()
                    .map(move |(i, _)| format!("chunk_{}_note_{}", r.patient_id, i))
            })
            .collect();

        EvalRecord {
            eval_id: format!("E{:09}", idx),
            query: self.query,
            answer: self.answer,
            ground_truth_patient_ids: self
                .matches
                .iter()
                .map(|r| r.patient_id.clone())
                .collect(),
            ground_truth_facts: self.facts,
            contexts,
            context_chunk_ids,
            query_type: self.query_type,
            difficulty: self.difficulty,
            metadata: EvalMetadata { seed: 0, batch_id: 0 },
        }
    }
}

fn pick<'a, T>(rng: &mut ChaCha8Rng, items: &'a [T]) -> &'a T {
    &items[rng.gen_range(0..items.len())]
}

/// Picks two distinct items. Returns `None` if there aren't two to pick from.
fn pick_pair<'a, T>(rng: &mut ChaCha8Rng, items: &'a [T]) -> Option<(&'a T, &'a T)> {
    if items.len() < 2 {
        return None;
    }
    let a = rng.gen_range(0..items.len());
    let mut b = rng.gen_range(0..items.len() - 1);
    if b >= a {
        b += 1;
    }
    Some((&items[a], &items[b]))
}

fn id_list(matches: &[&PatientRecord]) -> String {
    if matches.is_empty() {
        "None".to_string()
    } else {
        matches
            .iter()
            .map(|r| r.patient_id.clone())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn facts_from(matches: &[&PatientRecord], describe: impl Fn(&PatientRecord) -> String) -> Vec<GroundTruthFact> {
    matches
        .iter()
        .map(|r| GroundTruthFact {
            patient_id: r.patient_id.clone(),
            fact: describe(r),
        })
        .collect()
}

fn aggregate_fact(fact: String) -> Vec<GroundTruthFact> {
    vec![GroundTruthFact {
        patient_id: "_aggregate".to_string(),
        fact,
    }]
}

fn gender_word(gender: &Gender) -> &'static str {
    match gender {
        Gender::Female => "female",
        Gender::Male => "male",
    }
}

fn random_gender(rng: &mut ChaCha8Rng) -> Gender {
    if rng.gen::<f64>() < 0.5 {
        Gender::Female
    } else {
        Gender::Male
    }
}

fn reaction_of(r: &PatientRecord) -> &str {
    r.reaction_type.as_deref().unwrap_or("unknown")
}

pub fn generate_eval_record(
    rng: &mut ChaCha8Rng,
    ctx: &EvalContext,
    eval_index: u64,
    config: &EvalsConfig,
    metadata: EvalMetadata,
) -> Option<EvalRecord> {
    let query_type = pick_query_type(rng, config);
    let record = match query_type {
        EvalQueryType::Lookup => generate_lookup(rng, ctx, eval_index),
        EvalQueryType::FilteredLookup => generate_filtered_lookup(rng, ctx, eval_index),
        EvalQueryType::Aggregation => generate_aggregation(rng, ctx, eval_index),
        EvalQueryType::MultiHop => generate_multihop(rng, ctx, eval_index),
        EvalQueryType::Negative => generate_negative(rng, ctx, eval_index),
        EvalQueryType::Comparative => generate_comparative(rng, ctx, eval_index),
        EvalQueryType::EvidenceRetrieval => generate_evidence(rng, ctx, eval_index),
    };

    record.map(|mut r| {
        r.metadata = metadata;
        r
    })
}

/// Samples a query type from the enabled set by weight. Disabled types are removed from the
/// pool rather than falling through to their neighbour.
fn pick_query_type(rng: &mut ChaCha8Rng, config: &EvalsConfig) -> EvalQueryType {
    let mut pool: Vec<(EvalQueryType, f64)> = vec![
        (EvalQueryType::Lookup, 0.12),
        (EvalQueryType::FilteredLookup, 0.13),
        (EvalQueryType::Comparative, 0.15),
        (EvalQueryType::EvidenceRetrieval, 0.20),
    ];
    if config.include_aggregation_queries {
        pool.push((EvalQueryType::Aggregation, 0.15));
    }
    if config.include_multihop_queries {
        pool.push((EvalQueryType::MultiHop, 0.15));
    }
    if config.include_negative_controls {
        pool.push((EvalQueryType::Negative, 0.10));
    }

    let total: f64 = pool.iter().map(|(_, w)| w).sum();
    let mut roll: f64 = rng.gen::<f64>() * total;
    for (query_type, weight) in &pool {
        roll -= weight;
        if roll <= 0.0 {
            return query_type.clone();
        }
    }
    EvalQueryType::Lookup
}

/// Re-rolls a variant a few times to avoid emitting a query whose answer is trivially
/// empty. Empty answers still get through when the corpus genuinely has no match, which
/// keeps them possible without letting them dominate the class.
fn prefer_non_empty<'a, T>(
    rng: &mut ChaCha8Rng,
    attempts: usize,
    mut variant: impl FnMut(&mut ChaCha8Rng) -> (T, Vec<&'a PatientRecord>),
) -> (T, Vec<&'a PatientRecord>) {
    let mut chosen = variant(rng);
    for _ in 1..attempts {
        if !chosen.1.is_empty() {
            break;
        }
        chosen = variant(rng);
    }
    chosen
}

fn generate_lookup(rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let (query, matches) = prefer_non_empty(rng, 4, |rng| lookup_variant(rng, ctx));

    let facts = facts_from(&matches, |r| {
        format!(
            "reaction {}, severity {:?}",
            reaction_of(r),
            r.reaction_severity
        )
    });

    Some(
        EvalDraft {
            query,
            answer: id_list(&matches),
            matches,
            facts,
            query_type: EvalQueryType::Lookup,
            difficulty: Difficulty::Easy,
            context_limit: None,
        }
        .build(idx),
    )
}

fn lookup_variant<'a>(
    rng: &mut ChaCha8Rng,
    ctx: &'a EvalContext,
) -> (String, Vec<&'a PatientRecord>) {
    let med = *pick(rng, REACTION_DRUGS);
    let reaction_types = ctx.reaction_types();

    match rng.gen_range(0..3) {
        0 => (
            format!("Which patients had a reaction to {}?", med),
            ctx.reactors_to(med),
        ),
        1 => (
            format!("Which patients had a severe reaction to {}?", med),
            ctx.severe_reactors_to(med),
        ),
        _ if !reaction_types.is_empty() => {
            let reaction = pick(rng, &reaction_types).clone();
            (
                format!("Which patients experienced {} after taking {}?", reaction, med),
                ctx.reactors_with_type(med, &reaction),
            )
        }
        _ => (
            format!("Which patients had a reaction to {}?", med),
            ctx.reactors_to(med),
        ),
    }
}

fn generate_filtered_lookup(
    rng: &mut ChaCha8Rng,
    ctx: &EvalContext,
    idx: u64,
) -> Option<EvalRecord> {
    let med = *pick(rng, REACTION_DRUGS);
    let regions = ctx.regions();

    let (query, matches, descriptor) = match rng.gen_range(0..4) {
        0 => {
            let min_age: u8 = rng.gen_range(50..=75);
            let gender = random_gender(rng);
            let word = gender_word(&gender);
            (
                format!(
                    "Which {} patients over {} had a reaction to {}?",
                    word, min_age, med
                ),
                ctx.reactors_to(med)
                    .into_iter()
                    .filter(|r| r.age > min_age && r.gender == gender)
                    .collect::<Vec<_>>(),
                format!("age > {}, gender {}", min_age, word),
            )
        }
        1 if !regions.is_empty() => {
            let region = pick(rng, &regions).clone();
            (
                format!(
                    "Which patients in the {} region had a reaction to {}?",
                    region, med
                ),
                ctx.reactors_to(med)
                    .into_iter()
                    .filter(|r| r.region == region)
                    .collect::<Vec<_>>(),
                format!("region {}", region),
            )
        }
        2 => {
            let condition = *pick(rng, CONDITIONS);
            (
                format!(
                    "Which patients with {} had a reaction to {}?",
                    condition, med
                ),
                ctx.reactors_to(med)
                    .into_iter()
                    .filter(|r| r.comorbidities.iter().any(|c| c == condition))
                    .collect::<Vec<_>>(),
                format!("comorbidity {}", condition),
            )
        }
        _ => {
            let max_age: u8 = rng.gen_range(40..=65);
            (
                format!(
                    "Which patients under {} had a severe reaction to {}?",
                    max_age, med
                ),
                ctx.severe_reactors_to(med)
                    .into_iter()
                    .filter(|r| r.age < max_age)
                    .collect::<Vec<_>>(),
                format!("age < {}, severity Severe", max_age),
            )
        }
    };

    let facts = facts_from(&matches, |r| {
        format!(
            "{}, medication {}, reaction {}",
            descriptor,
            med,
            reaction_of(r)
        )
    });

    Some(
        EvalDraft {
            query,
            answer: id_list(&matches),
            matches,
            facts,
            query_type: EvalQueryType::FilteredLookup,
            difficulty: Difficulty::Medium,
            context_limit: None,
        }
        .build(idx),
    )
}

fn generate_aggregation(rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let (query, answer, matches, fact) = match rng.gen_range(0..5) {
        0 => {
            let med = *pick(rng, REACTION_DRUGS);
            let exposed = ctx.exposed_to(med);
            let reactors = ctx.reactors_to(med);
            let pct = if exposed.is_empty() {
                0.0
            } else {
                (reactors.len() as f64 / exposed.len() as f64) * 100.0
            };
            let fact = format!(
                "{} exposed: {}, reactors: {}, severe: {}",
                med,
                exposed.len(),
                reactors.len(),
                ctx.severe_reactors_to(med).len()
            );
            (
                format!(
                    "What percentage of {} patients had an adverse reaction?",
                    med
                ),
                format!(
                    "{:.1}% ({} out of {} exposed)",
                    pct,
                    reactors.len(),
                    exposed.len()
                ),
                reactors,
                fact,
            )
        }
        1 => {
            let drug = *pick(rng, ALL_DRUGS);
            let exposed = ctx.exposed_to(drug);
            let fact = format!("{} prescriptions: {}", drug, exposed.len());
            (
                format!("How many patients were prescribed {}?", drug),
                format!("{}", exposed.len()),
                exposed,
                fact,
            )
        }
        2 => {
            let condition = *pick(rng, CONDITIONS);
            let cohort = ctx.with_condition(condition);
            let answer = if cohort.is_empty() {
                "None".to_string()
            } else {
                let mean =
                    cohort.iter().map(|r| r.age as f64).sum::<f64>() / cohort.len() as f64;
                format!("{:.1}", mean)
            };
            let fact = format!("{} cohort size: {}", condition, cohort.len());
            (
                format!("What is the average age of patients with {}?", condition),
                answer,
                cohort,
                fact,
            )
        }
        3 => {
            let condition = *pick(rng, CONDITIONS);
            let cohort: Vec<_> = ctx
                .with_condition(condition)
                .into_iter()
                .filter(|r| matches!(r.reaction_severity, Some(Severity::Severe)))
                .collect();
            let fact = format!("{} patients with a severe reaction: {}", condition, cohort.len());
            (
                format!(
                    "How many patients with {} had a severe adverse reaction?",
                    condition
                ),
                format!("{}", cohort.len()),
                cohort,
                fact,
            )
        }
        _ => {
            let condition = *pick(rng, CONDITIONS);
            let gender = random_gender(rng);
            let word = gender_word(&gender);
            let cohort: Vec<_> = ctx
                .with_condition(condition)
                .into_iter()
                .filter(|r| r.gender == gender)
                .collect();
            let fact = format!("{} {} patients: {}", word, condition, cohort.len());
            (
                format!("How many {} patients have {}?", word, condition),
                format!("{}", cohort.len()),
                cohort,
                fact,
            )
        }
    };

    Some(
        EvalDraft {
            query,
            answer,
            matches,
            facts: aggregate_fact(fact),
            query_type: EvalQueryType::Aggregation,
            difficulty: Difficulty::Medium,
            context_limit: Some(AGGREGATE_CONTEXT_LIMIT),
        }
        .build(idx),
    )
}

fn generate_multihop(rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let ((query, descriptor), matches) =
        prefer_non_empty(rng, 4, |rng| multihop_variant(rng, ctx));

    let answer = format!("{}", matches.len());
    let facts = facts_from(&matches, |r| {
        format!("{}, reaction {}", descriptor, reaction_of(r))
    });

    Some(
        EvalDraft {
            query,
            answer,
            matches,
            facts,
            query_type: EvalQueryType::MultiHop,
            difficulty: Difficulty::Hard,
            context_limit: None,
        }
        .build(idx),
    )
}

fn multihop_variant<'a>(
    rng: &mut ChaCha8Rng,
    ctx: &'a EvalContext,
) -> ((String, String), Vec<&'a PatientRecord>) {
    let med = *pick(rng, REACTION_DRUGS);

    let (query, matches, descriptor) = match rng.gen_range(0..3) {
        0 => {
            let condition = *pick(rng, CONDITIONS);
            (
                format!(
                    "Among {} patients prescribed {}, how many had an adverse reaction?",
                    condition, med
                ),
                ctx.reactors_to(med)
                    .into_iter()
                    .filter(|r| r.comorbidities.iter().any(|c| c == condition))
                    .collect::<Vec<_>>(),
                format!("{}, {}, reaction", condition, med),
            )
        }
        1 => {
            let (cond_a, cond_b) = pick_pair(rng, CONDITIONS).expect("CONDITIONS has >= 2 entries");
            (
                format!(
                    "Among patients with both {} and {}, how many were prescribed {}?",
                    cond_a, cond_b, med
                ),
                ctx.exposed_to(med)
                    .into_iter()
                    .filter(|r| {
                        r.comorbidities.iter().any(|c| c == cond_a)
                            && r.comorbidities.iter().any(|c| c == cond_b)
                    })
                    .collect::<Vec<_>>(),
                format!("{} + {}, prescribed {}", cond_a, cond_b, med),
            )
        }
        _ => {
            let min_age: u8 = rng.gen_range(45..=70);
            let condition = *pick(rng, CONDITIONS);
            (
                format!(
                    "How many patients over {} with {} had a severe reaction to {}?",
                    min_age, condition, med
                ),
                ctx.severe_reactors_to(med)
                    .into_iter()
                    .filter(|r| {
                        r.age > min_age && r.comorbidities.iter().any(|c| c == condition)
                    })
                    .collect::<Vec<_>>(),
                format!("age > {}, {}, severe reaction to {}", min_age, condition, med),
            )
        }
    };

    ((query, descriptor), matches)
}

/// Negative controls. Every probe is verified against the corpus before the record claims
/// "None" - a probe that unexpectedly matches falls back to a guaranteed-absent drug.
fn generate_negative(rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let (query, matched) = match rng.gen_range(0..4) {
        0 => {
            let drug = *pick(rng, UNKNOWN_DRUGS);
            (
                format!("Which patients had a reaction to {}?", drug),
                !ctx.exposed_to(drug).is_empty(),
            )
        }
        1 => {
            let condition = *pick(rng, UNKNOWN_CONDITIONS);
            (
                format!("Which patients were diagnosed with {}?", condition),
                !ctx.with_condition(condition).is_empty(),
            )
        }
        2 => {
            // A real drug that the reaction model never produces reactions for.
            let drug = *pick(rng, ALL_DRUGS);
            (
                format!("Which patients had a documented reaction to {}?", drug),
                !ctx.reactors_to(drug).is_empty(),
            )
        }
        _ => {
            let fake_id = format!("P{:09}", 900_000_000 + rng.gen_range(0..1_000_000u64));
            (
                format!("What medications was patient {} prescribed?", fake_id),
                ctx.find_by_id(&fake_id).is_some(),
            )
        }
    };

    // The probe hit real data, so it is not a negative control. Fall back to a drug that is
    // absent from the corpus by construction.
    let query = if matched {
        let drug = *pick(rng, UNKNOWN_DRUGS);
        format!("Which patients had a reaction to {}?", drug)
    } else {
        query
    };

    Some(
        EvalDraft {
            query,
            answer: "None".to_string(),
            matches: Vec::new(),
            facts: Vec::new(),
            query_type: EvalQueryType::Negative,
            difficulty: Difficulty::Easy,
            context_limit: None,
        }
        .build(idx),
    )
}

fn generate_comparative(rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let (query, answer, matches, facts) = match rng.gen_range(0..4) {
        0 => {
            let (a, b) = pick_pair(rng, ALL_DRUGS).expect("ALL_DRUGS has >= 2 entries");
            let a_reactors = ctx.reactors_to(a);
            let b_reactors = ctx.reactors_to(b);
            let answer = compare_counts(a, a_reactors.len(), b, b_reactors.len(), "reactions");
            let facts = vec![
                GroundTruthFact {
                    patient_id: "_comparative".to_string(),
                    fact: format!("{} reactions: {}", a, a_reactors.len()),
                },
                GroundTruthFact {
                    patient_id: "_comparative".to_string(),
                    fact: format!("{} reactions: {}", b, b_reactors.len()),
                },
            ];
            let matches = a_reactors.into_iter().chain(b_reactors).collect::<Vec<_>>();
            (
                format!(
                    "Was {} or {} associated with more adverse reactions?",
                    a, b
                ),
                answer,
                matches,
                facts,
            )
        }
        1 => {
            let (a, b) = pick_pair(rng, CONDITIONS).expect("CONDITIONS has >= 2 entries");
            let a_cohort = ctx.with_condition(a);
            let b_cohort = ctx.with_condition(b);
            let answer = compare_counts(a, a_cohort.len(), b, b_cohort.len(), "patients");
            let facts = vec![
                GroundTruthFact {
                    patient_id: "_comparative".to_string(),
                    fact: format!("{} patients: {}", a, a_cohort.len()),
                },
                GroundTruthFact {
                    patient_id: "_comparative".to_string(),
                    fact: format!("{} patients: {}", b, b_cohort.len()),
                },
            ];
            let matches = a_cohort.into_iter().chain(b_cohort).collect::<Vec<_>>();
            (
                format!("Which is more common in the cohort: {} or {}?", a, b),
                answer,
                matches,
                facts,
            )
        }
        2 => {
            let condition = *pick(rng, CONDITIONS);
            let with: Vec<_> = ctx.with_condition(condition);
            let with_rate = reaction_rate(&with);
            let without: Vec<_> = ctx
                .records
                .iter()
                .filter(|r| !r.comorbidities.iter().any(|c| c == condition))
                .collect();
            let without_rate = reaction_rate(&without);
            let answer = if with_rate > without_rate {
                format!(
                    "Patients with {} ({:.1}% vs {:.1}%)",
                    condition, with_rate, without_rate
                )
            } else if without_rate > with_rate {
                format!(
                    "Patients without {} ({:.1}% vs {:.1}%)",
                    condition, without_rate, with_rate
                )
            } else {
                format!("Equal ({:.1}% each)", with_rate)
            };
            let facts = vec![
                GroundTruthFact {
                    patient_id: "_comparative".to_string(),
                    fact: format!(
                        "{} cohort: {} patients, {:.1}% reaction rate",
                        condition,
                        with.len(),
                        with_rate
                    ),
                },
                GroundTruthFact {
                    patient_id: "_comparative".to_string(),
                    fact: format!(
                        "non-{} cohort: {} patients, {:.1}% reaction rate",
                        condition,
                        without.len(),
                        without_rate
                    ),
                },
            ];
            let matches: Vec<_> = with
                .into_iter()
                .filter(|r| r.allergic_reaction)
                .collect();
            (
                format!(
                    "Do patients with {} have a higher adverse reaction rate than those without?",
                    condition
                ),
                answer,
                matches,
                facts,
            )
        }
        _ => {
            let med = *pick(rng, REACTION_DRUGS);
            let cutoff: u8 = rng.gen_range(40..=70);
            let reactors = ctx.reactors_to(med);
            let younger = reactors.iter().filter(|r| r.age < cutoff).count();
            let older = reactors.len() - younger;
            let answer = if younger > older {
                format!("Under {} ({} vs {})", cutoff, younger, older)
            } else if older > younger {
                format!("{} and older ({} vs {})", cutoff, older, younger)
            } else {
                format!("Equal ({} each)", younger)
            };
            let facts = vec![
                GroundTruthFact {
                    patient_id: "_comparative".to_string(),
                    fact: format!("{} reactors under {}: {}", med, cutoff, younger),
                },
                GroundTruthFact {
                    patient_id: "_comparative".to_string(),
                    fact: format!("{} reactors {} and older: {}", med, cutoff, older),
                },
            ];
            (
                format!(
                    "Which age group had more reactions to {}: patients under {} or {} and older?",
                    med, cutoff, cutoff
                ),
                answer,
                reactors,
                facts,
            )
        }
    };

    Some(
        EvalDraft {
            query,
            answer,
            matches,
            facts,
            query_type: EvalQueryType::Comparative,
            difficulty: Difficulty::Medium,
            context_limit: Some(AGGREGATE_CONTEXT_LIMIT),
        }
        .build(idx),
    )
}

fn compare_counts(a: &str, a_count: usize, b: &str, b_count: usize, noun: &str) -> String {
    if a_count > b_count {
        format!("{} ({} {} vs {} for {})", a, a_count, noun, b_count, b)
    } else if b_count > a_count {
        format!("{} ({} {} vs {} for {})", b, b_count, noun, a_count, a)
    } else {
        format!("Equal ({} {} each)", a_count, noun)
    }
}

fn reaction_rate(cohort: &[&PatientRecord]) -> f64 {
    if cohort.is_empty() {
        return 0.0;
    }
    let reactors = cohort.iter().filter(|r| r.allergic_reaction).count();
    (reactors as f64 / cohort.len() as f64) * 100.0
}

fn generate_evidence(rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    // Each variant needs a non-empty pool; fall through to the next until one has records.
    let variant = rng.gen_range(0..3);
    let attempt = |variant: usize, rng: &mut ChaCha8Rng| -> Option<(String, Vec<&PatientRecord>, String)> {
        match variant {
            0 => {
                let med = *pick(rng, REACTION_DRUGS);
                let pool = ctx.reactors_to(med);
                (!pool.is_empty()).then(|| {
                    (
                        format!("reaction to {}", med),
                        pool,
                        format!("{} reaction", med),
                    )
                })
            }
            1 => {
                let condition = *pick(rng, CONDITIONS);
                let pool = ctx.with_condition(condition);
                (!pool.is_empty()).then(|| {
                    (
                        format!("diagnosis of {}", condition),
                        pool,
                        format!("comorbidity {}", condition),
                    )
                })
            }
            _ => {
                let drug = *pick(rng, ALL_DRUGS);
                let pool = ctx.exposed_to(drug);
                (!pool.is_empty()).then(|| {
                    (
                        format!("prescription of {}", drug),
                        pool,
                        format!("medication {}", drug),
                    )
                })
            }
        }
    };

    let (subject, pool, fact) = (0..3)
        .find_map(|offset| attempt((variant + offset) % 3, rng))
        .or_else(|| {
            (!ctx.records.is_empty()).then(|| {
                (
                    "this patient's history".to_string(),
                    ctx.records.iter().collect::<Vec<_>>(),
                    "patient record".to_string(),
                )
            })
        })?;

    let patient = pool[rng.gen_range(0..pool.len())];
    let note_text = patient
        .clinical_notes
        .first()
        .map(|n| n.text.clone())
        .unwrap_or_default();

    Some(
        EvalDraft {
            query: format!(
                "Find the clinical note documenting the {} for patient {}.",
                subject, patient.patient_id
            ),
            answer: note_text,
            matches: vec![patient],
            facts: vec![GroundTruthFact {
                patient_id: patient.patient_id.clone(),
                fact,
            }],
            query_type: EvalQueryType::EvidenceRetrieval,
            difficulty: Difficulty::Easy,
            context_limit: None,
        }
        .build(idx),
    )
}

pub fn to_ragas_record(eval: &EvalRecord) -> RagasRecord {
    RagasRecord {
        question: eval.query.clone(),
        answer: eval.answer.clone(),
        contexts: eval.contexts.clone(),
        // `answer` is the reference answer computed from the corpus, so it is the ground
        // truth for every query class - including the ones whose answer is a count,
        // percentage, drug name, or note text rather than a list of patient ids.
        ground_truth: eval.answer.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::clinical_note::ClinicalNote;
    use crate::domain::patient::{PatientMetadata, PatientName};

    fn record(id: &str, age: u8, reacted_to: Option<&str>) -> PatientRecord {
        PatientRecord {
            patient_id: id.to_string(),
            name: PatientName::new("Test".to_string(), "Patient".to_string()),
            age,
            gender: Gender::Female,
            region: "Northeast".to_string(),
            comorbidities: vec!["diabetes".to_string()],
            medications: vec!["DrugX".to_string(), "DrugY".to_string()],
            allergic_reaction: reacted_to.is_some(),
            reaction_medication: reacted_to.map(String::from),
            reaction_type: reacted_to.map(|_| "rash".to_string()),
            reaction_severity: reacted_to.map(|_| Severity::Severe),
            clinical_notes: vec![ClinicalNote {
                note_id: format!("note_{}", id),
                patient_id: id.to_string(),
                note_type: "primary_care".to_string(),
                text: format!("Note for {}", id),
            }],
            metadata: PatientMetadata { seed: 0, batch_id: 0 },
        }
    }

    fn context() -> EvalContext {
        EvalContext {
            records: (0..40)
                .map(|i| {
                    let id = format!("P{:09}", i);
                    let reacted = if i % 3 == 0 { Some("DrugX") } else { None };
                    record(&id, 30 + (i % 50) as u8, reacted)
                })
                .collect(),
        }
    }

    fn generate_all(count: u64) -> Vec<EvalRecord> {
        let ctx = context();
        let config = EvalsConfig::default();
        let mut rng = crate::rng::batch_rng(42, 0);
        (0..count)
            .filter_map(|i| {
                let mut prng = crate::rng::patient_rng(&mut rng, i);
                generate_eval_record(
                    &mut prng,
                    &ctx,
                    i,
                    &config,
                    EvalMetadata { seed: 42, batch_id: 7 },
                )
            })
            .collect()
    }

    #[test]
    fn every_record_carries_its_batch_metadata() {
        for eval in generate_all(200) {
            assert_eq!(eval.metadata.seed, 42);
            assert_eq!(eval.metadata.batch_id, 7);
        }
    }

    #[test]
    fn all_seven_query_classes_are_generated() {
        let seen: std::collections::HashSet<_> = generate_all(500)
            .into_iter()
            .map(|e| e.query_type)
            .collect();
        for expected in [
            EvalQueryType::Lookup,
            EvalQueryType::FilteredLookup,
            EvalQueryType::Aggregation,
            EvalQueryType::MultiHop,
            EvalQueryType::Negative,
            EvalQueryType::Comparative,
            EvalQueryType::EvidenceRetrieval,
        ] {
            assert!(seen.contains(&expected), "missing query class {:?}", expected);
        }
    }

    #[test]
    fn negative_controls_have_no_supporting_evidence() {
        let negatives: Vec<_> = generate_all(500)
            .into_iter()
            .filter(|e| e.query_type == EvalQueryType::Negative)
            .collect();
        assert!(!negatives.is_empty());
        for eval in negatives {
            assert_eq!(eval.answer, "None");
            assert!(eval.ground_truth_patient_ids.is_empty());
            assert!(eval.contexts.is_empty());
            assert!(eval.context_chunk_ids.is_empty());
        }
    }

    #[test]
    fn disabled_classes_are_not_generated() {
        let ctx = context();
        let config = EvalsConfig {
            include_negative_controls: false,
            include_aggregation_queries: false,
            include_multihop_queries: false,
            ..EvalsConfig::default()
        };
        let mut rng = crate::rng::batch_rng(42, 0);
        for i in 0..500 {
            let mut prng = crate::rng::patient_rng(&mut rng, i);
            let eval = generate_eval_record(
                &mut prng,
                &ctx,
                i,
                &config,
                EvalMetadata { seed: 42, batch_id: 0 },
            )
            .unwrap();
            assert!(
                !matches!(
                    eval.query_type,
                    EvalQueryType::Negative
                        | EvalQueryType::Aggregation
                        | EvalQueryType::MultiHop
                ),
                "disabled class {:?} was generated",
                eval.query_type
            );
        }
    }

    #[test]
    fn list_answers_match_their_ground_truth_ids() {
        for eval in generate_all(500) {
            if matches!(
                eval.query_type,
                EvalQueryType::Lookup | EvalQueryType::FilteredLookup
            ) {
                let expected = if eval.ground_truth_patient_ids.is_empty() {
                    "None".to_string()
                } else {
                    eval.ground_truth_patient_ids.join(", ")
                };
                assert_eq!(eval.answer, expected, "query: {}", eval.query);
            }
        }
    }

    #[test]
    fn ragas_ground_truth_is_the_reference_answer() {
        for eval in generate_all(200) {
            assert_eq!(to_ragas_record(&eval).ground_truth, eval.answer);
        }
    }

    #[test]
    fn generation_is_deterministic_for_a_given_seed() {
        let first: Vec<_> = generate_all(100).into_iter().map(|e| e.query).collect();
        let second: Vec<_> = generate_all(100).into_iter().map(|e| e.query).collect();
        assert_eq!(first, second);
    }
}
