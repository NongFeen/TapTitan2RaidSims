use rand::RngCore;

/// A pluggable source of randomness for the tap-by-tap proc rolls in
/// `SimService::run_deck_sim`. `AlwaysSucceed` always returns 0 -- since a
/// proc fires whenever `roll <= proc_chance`, this makes every card with a
/// nonzero proc chance fire on every roll it gets. Used only by the
/// deterministic "sim-to-real" golden tests (see
/// `run_deck_debug_simulation`'s `force_guaranteed_procs` flag) to remove
/// RNG variance from the comparison against a real, human-measured TT2
/// attack -- real gameplay and every other caller keep using `Real`.
pub(super) enum SimRng {
    Real(rand::rngs::ThreadRng),
    AlwaysSucceed,
}

impl RngCore for SimRng {
    fn next_u32(&mut self) -> u32 {
        match self {
            Self::Real(rng) => rng.next_u32(),
            Self::AlwaysSucceed => 0,
        }
    }

    fn next_u64(&mut self) -> u64 {
        match self {
            Self::Real(rng) => rng.next_u64(),
            Self::AlwaysSucceed => 0,
        }
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        match self {
            Self::Real(rng) => rng.fill_bytes(dst),
            Self::AlwaysSucceed => dst.fill(0),
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/services/taptitan/sim_service/deterministic_rng_tests.rs"]
mod tests;
