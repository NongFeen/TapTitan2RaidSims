use super::*;
use rand::Rng;

#[test]
fn always_succeed_rolls_the_minimum_possible_value() {
    let mut rng = SimRng::AlwaysSucceed;
    // `tap_boss` fires a proc whenever `roll <= proc_chance`; a roll of 0.0
    // guarantees that for any proc_chance > 0.0, which is the whole point.
    let roll: f64 = rng.random();
    assert_eq!(roll, 0.0);
    // Exercise the other RngCore entry points too, since `Rng::random`'s
    // internal path can vary by output type.
    assert_eq!(rng.next_u32(), 0);
    assert_eq!(rng.next_u64(), 0);
    let mut bytes = [0xFFu8; 8];
    rng.fill_bytes(&mut bytes);
    assert_eq!(bytes, [0u8; 8]);
}
