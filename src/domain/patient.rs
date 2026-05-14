use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Gender {
    Female,
    Male,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Severity {
    Mild,
    Moderate,
    Severe,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RiskBucket {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PatientName {
    pub first_name: String,
    pub last_name: String,
    pub middle_initial: Option<char>,
    pub name_suffix: Option<String>,
}

impl PatientName {
    pub fn new(first: String, last: String) -> Self {
        Self {
            first_name: first,
            last_name: last,
            middle_initial: None,
            name_suffix: None,
        }
    }

    pub fn with_middle(mut self, initial: char) -> Self {
        self.middle_initial = Some(initial);
        self
    }

    pub fn with_suffix(mut self, suffix: String) -> Self {
        self.name_suffix = Some(suffix);
        self
    }

    /// Returns full name formatted as "First M. Last, Suffix"
    pub fn full(&self) -> String {
        match (&self.middle_initial, &self.name_suffix) {
            (Some(mi), Some(suffix)) => {
                format!("{} {}. {}, {}", self.first_name, mi, self.last_name, suffix)
            }
            (Some(mi), None) => {
                format!("{} {}. {}", self.first_name, mi, self.last_name)
            }
            (None, Some(suffix)) => {
                format!("{} {}, {}", self.first_name, self.last_name, suffix)
            }
            (None, None) => {
                format!("{} {}", self.first_name, self.last_name)
            }
        }
    }

    /// Returns sortable name as "Last, First M."
    pub fn sort_key(&self) -> String {
        match &self.middle_initial {
            Some(mi) => format!("{}, {} {}", self.last_name, self.first_name, mi),
            None => format!("{}, {}", self.last_name, self.first_name),
        }
    }

    /// Returns initials as "J. Smith"
    pub fn initials(&self) -> String {
        let first_init = self.first_name.chars().next().unwrap_or(' ');
        match &self.middle_initial {
            Some(mi) => format!("{}. {}. {}", first_init, mi, self.last_name),
            None => format!("{}. {}", first_init, self.last_name),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientProfile {
    pub patient_id: String,
    pub name: PatientName,
    pub age: u8,
    pub gender: Gender,
    pub region: String,
    pub risk_bucket: RiskBucket,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientWithConditions {
    pub profile: PatientProfile,
    pub comorbidities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientWithMedications {
    pub profile: PatientProfile,
    pub comorbidities: Vec<String>,
    pub medications: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientRecordDraft {
    pub profile: PatientProfile,
    pub comorbidities: Vec<String>,
    pub medications: Vec<String>,
    pub allergic_reaction: bool,
    pub reaction_medication: Option<String>,
    pub reaction_type: Option<String>,
    pub reaction_severity: Option<Severity>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientMetadata {
    pub seed: u64,
    pub batch_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientRecord {
    pub patient_id: String,
    pub name: PatientName,
    pub age: u8,
    pub gender: Gender,
    pub region: String,
    pub comorbidities: Vec<String>,
    pub medications: Vec<String>,
    pub allergic_reaction: bool,
    pub reaction_medication: Option<String>,
    pub reaction_type: Option<String>,
    pub reaction_severity: Option<Severity>,
    pub clinical_notes: Vec<super::clinical_note::ClinicalNote>,
    pub metadata: PatientMetadata,
}
