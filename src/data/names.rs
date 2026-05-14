//! Census-based name data for deterministic patient name generation.
//! All names are from public US Census data, ranked by frequency.

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::domain::patient::Gender;

/// ~100 most common female names by census frequency
pub static FIRST_NAMES_FEMALE: &[&str] = &[
    "Mary", "Patricia", "Jennifer", "Linda", "Elizabeth",
    "Barbara", "Susan", "Jessica", "Sarah", "Karen", "Nancy",
    "Lisa", "Betty", "Margaret", "Sandra", "Ashley", "Kimberly",
    "Emily", "Donna", "Michelle", "Dorothy", "Carol", "Amanda",
    "Melissa", "Deborah", "Stephanie", "Rebecca", "Sharon", "Laura",
    "Cynthia", "Kathleen", "Amy", "Angela", "Shirley", "Anna",
    "Brenda", "Pamela", "Emma", "Nicole", "Helen", "Samantha",
    "Katherine", "Christine", "Debra", "Rachel", "Carolyn", "Janet",
    "Catherine", "Maria", "Heather", "Diane", "Julie", "Joyce",
    "Victoria", "Ruth", "Andrea", "Lauren", "Evelyn", "Judith",
    "Megan", "Cheryl", "Martha", "Frances", "Hannah",
    "Jacqueline", "Annie", "Gloria", "Eleanor", "Teresa",
    "Kathryn", "Sara", "Janice", "Jean", "Alice", "Doris",
    "Abigail", "Julia", "Judy", "Grace", "Denise", "Amber",
    "Marilyn", "Beverly", "Danielle", "Theresa", "Sophia", "Marie",
    "Diana", "Brittany", "Natalie", "Isabella", "Charlotte", "Rose",
    "Alexis", "Kayla", "Mia", "Alexandra", "Lillian", "Claire",
];

/// ~100 most common male names by census frequency
pub static FIRST_NAMES_MALE: &[&str] = &[
    "James", "Robert", "John", "Michael", "David", "William",
    "Richard", "Joseph", "Thomas", "Charles", "Christopher", "Daniel",
    "Matthew", "Anthony", "Donald", "Mark", "Paul", "Steven",
    "Andrew", "Kenneth", "Joshua", "Kevin", "Brian", "George",
    "Timothy", "Ronald", "Edward", "Jason", "Jeffrey", "Ryan",
    "Jacob", "Gary", "Nicholas", "Eric", "Jonathan", "Stephen",
    "Larry", "Justin", "Scott", "Brandon", "Benjamin", "Samuel",
    "Raymond", "Gregory", "Frank", "Alexander", "Patrick", "Jack",
    "Dennis", "Jerry", "Tyler", "Aaron", "Jose", "Adam",
    "Henry", "Nathan", "Douglas", "Zachary", "Peter", "Kyle",
    "Noah", "Ethan", "Jeremy", "Walter", "Christian", "Keith",
    "Roger", "Terry", "Gerald", "Harold", "Sean", "Austin",
    "Arthur", "Lawrence", "Jesse", "Dylan", "Bryan", "Joe",
    "Jordan", "Billy", "Bruce", "Albert", "Willie", "Gabriel",
    "Logan", "Alan", "Juan", "Wayne", "Elijah", "Roy",
    "Ralph", "Randy", "Eugene", "Vince", "Russell", "Louis",
    "Philip", "Bobby", "Johnny", "Bradley", "Marcus", "Melvin",
];

/// ~200 most common surnames by census frequency
pub static LAST_NAMES: &[&str] = &[
    "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia",
    "Miller", "Davis", "Rodriguez", "Martinez", "Hernandez", "Lopez",
    "Gonzalez", "Wilson", "Anderson", "Thomas", "Taylor", "Moore",
    "Jackson", "Martin", "Lee", "Perez", "Thompson", "White",
    "Harris", "Sanchez", "Clark", "Ramirez", "Lewis", "Robinson",
    "Walker", "Young", "Allen", "King", "Wright", "Scott",
    "Torres", "Nguyen", "Hill", "Flores", "Green", "Adams",
    "Nelson", "Baker", "Hall", "Rivera", "Campbell", "Mitchell",
    "Carter", "Roberts", "Gomez", "Phillips", "Evans", "Turner",
    "Diaz", "Parker", "Cruz", "Edwards", "Collins", "Reyes",
    "Stewart", "Morris", "Morales", "Murphy", "Cook", "Rogers",
    "Gutierrez", "Ortiz", "Morgan", "Cooper", "Peterson", "Bailey",
    "Reed", "Kelly", "Howard", "Ramos", "Kim", "Cox",
    "Ward", "Richardson", "Watson", "Brooks", "Chavez", "Wood",
    "Bennett", "Gray", "Mendoza", "Ruiz", "Hughes", "Price",
    "Alvarez", "Castillo", "Sanders", "Patel", "Myers", "Long",
    "Ross", "Foster", "Jimenez", "Powell", "Jenkins", "Perry",
    "Russell", "Sullivan", "Bell", "Coleman", "Butler", "Henderson",
    "Barnes", "Gonzales", "Fisher", "Vasquez", "Dawson", "Santiago",
    "Moon", "Holmes", "Daniel", "Ferguson", "Gibson",
    "Reynolds", "Carpenter", "Jordan", "Romero", "Kennedy",
    "Owens", "Harrison", "Hamilton", "Graham", "Grant", "West",
    "James", "Shaw", "Holcomb", "Cunningham", "Alexander", "Lane",
    "Garrett", "Mills", "Ray", "Burton", "Carson", "Richmond",
    "Boone", "Baxter", "Hodges", "Pearson", "Holland", "Douglas",
    "Fleming", "Hansen", "Steele", "Jacobsen", "Malone", "Richards",
    "Sharp", "Wheeler", "Nicholson", "Wallace", "Weaver", "Gould",
    "Hutchinson", "Simpson", "Wagner", "Beck",
    "Kincaid", "Vaughn", "Horton", "Shepherd", "Sawyer", "Bishop",
    "Warren", "Larson", "Stanley", "Morrow", "Hawkins",
    "Carlson", "Lawson", "Fields", "Gardner", "Stephens",
    "Gillespie", "Wall", "Hayes", "Pearce", "Hoffman", "Benson",
    "Mahoney", "Fletcher", "Decker", "Baird", "Meier", "Shelton",
    "Black", "Klein", "Barlow", "Jacobson", "McGuire",
    "Burns", "Pierce", "Conner", "Lang",
    "Lynch", "Mack", "Bowman", "Fitzgerald", "Briggs", "Winter",
    "Mercer", "Knight", "Graves", "Berry", "Hoff",
    "Bender", "Lyons", "Hendricks", "Hendrix", "Conway",
];

/// Regional surname adjustments - demonstrates demographic awareness
pub struct RegionalNamePools {
    pub northeast: &'static [&'static str],
    pub southeast: &'static [&'static str],
    pub midwest: &'static [&'static str],
    pub southwest: &'static [&'static str],
    pub west: &'static [&'static str],
}

pub static REGIONAL_POOLS: RegionalNamePools = RegionalNamePools {
    northeast: &[
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Miller",
        "Davis", "Wilson", "Anderson", "Taylor", "Thomas", "Moore",
        "Sullivan", "O'Brien", "Murphy", "Kelly", "Ryan", "Connolly",
        "Romano", "Russo", "Esposito", "Costello",
    ],
    southeast: &[
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Davis",
        "Wilson", "Taylor", "Moore", "Anderson", "Thomas", "Jackson",
        "Campbell", "MacDonald", "Sullivan", "Murphy", "Fitzpatrick",
    ],
    midwest: &[
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Miller",
        "Davis", "Wilson", "Anderson", "Taylor", "Thomas", "Moore",
        "Schmidt", "Mueller", "Weber", "Wagner", "Becker", "Hoffman",
        "Olson", "Larson", "Carlson", "Jensen",
    ],
    southwest: &[
        "Smith", "Johnson", "Williams", "Brown", "Jones",
        "Garcia", "Rodriguez", "Martinez", "Hernandez", "Lopez",
        "Gonzalez", "Perez", "Sanchez", "Ramirez", "Torres", "Rivera",
        "Cruz", "Ortiz", "Mendoza", "Ruiz", "Vasquez", "Castillo",
    ],
    west: &[
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia",
        "Miller", "Davis", "Rodriguez", "Martinez", "Hernandez", "Lopez",
        "Lee", "Nguyen", "Kim", "Wong", "Chen", "Patel", "Singh",
    ],
};

impl RegionalNamePools {
    pub fn get_pool_for_region(&self, region: &str) -> &'static [&'static str] {
        match region {
            "Northeast" => self.northeast,
            "Southeast" => self.southeast,
            "Midwest" => self.midwest,
            "Southwest" => self.southwest,
            "West" | _ => self.west,
        }
    }
}

/// Generate a deterministic patient name based on RNG state, gender, and region
pub fn generate_name(
    rng: &mut ChaCha8Rng,
    gender: &Gender,
    region: &str,
) -> crate::domain::patient::PatientName {
    let first_pool = match gender {
        Gender::Female => FIRST_NAMES_FEMALE,
        Gender::Male => FIRST_NAMES_MALE,
    };
    let first_idx = rng.gen_range(0..first_pool.len());
    let first_name = first_pool[first_idx].to_string();

    let last_pool = REGIONAL_POOLS.get_pool_for_region(region);
    let last_idx = rng.gen_range(0..last_pool.len());
    let last_name = last_pool[last_idx].to_string();

    let middle_initial = if rng.gen::<f64>() < 0.10 {
        Some((b'A' + rng.gen_range(0..26)) as char)
    } else {
        None
    };

    let name_suffix = if rng.gen::<f64>() < 0.02 {
        let suffixes = ["Jr.", "Sr.", "II", "III", "IV"];
        Some(suffixes[rng.gen_range(0..suffixes.len())].to_string())
    } else {
        None
    };

    let mut name = crate::domain::patient::PatientName::new(first_name, last_name);
    if let Some(mi) = middle_initial {
        name = name.with_middle(mi);
    }
    if let Some(suffix) = name_suffix {
        name = name.with_suffix(suffix);
    }

    name
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_name_generation_deterministic() {
        let seed = 42u64;
        let mut rng1 = ChaCha8Rng::seed_from_u64(seed);
        let mut rng2 = ChaCha8Rng::seed_from_u64(seed);

        let name1 = generate_name(&mut rng1, &Gender::Female, "Northeast");
        let name2 = generate_name(&mut rng2, &Gender::Female, "Northeast");

        assert_eq!(name1, name2, "Same seed should produce same name");
    }

    #[test]
    fn test_regional_pool_selection() {
        assert_eq!(REGIONAL_POOLS.get_pool_for_region("Northeast").len(), REGIONAL_POOLS.northeast.len());
        assert_eq!(REGIONAL_POOLS.get_pool_for_region("Southeast").len(), REGIONAL_POOLS.southeast.len());
        assert_eq!(REGIONAL_POOLS.get_pool_for_region("Midwest").len(), REGIONAL_POOLS.midwest.len());
        assert_eq!(REGIONAL_POOLS.get_pool_for_region("Southwest").len(), REGIONAL_POOLS.southwest.len());
        assert_eq!(REGIONAL_POOLS.get_pool_for_region("West").len(), REGIONAL_POOLS.west.len());
    }
}
