use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn batch_rng(root_seed: u64, batch_id: u64) -> ChaCha8Rng {
    let seed = root_seed ^ batch_id;
    ChaCha8Rng::seed_from_u64(seed)
}

pub fn patient_rng(batch: &mut ChaCha8Rng, patient_index: u64) -> ChaCha8Rng {
    let seed = batch.gen::<u64>() ^ patient_index;
    ChaCha8Rng::seed_from_u64(seed)
}
