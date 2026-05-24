//! Reusable bust-detection state machine.
//!
//! Pattern: a bee detects its own bust (e.g. principal under a configured threshold), signs
//! a structured `LoanRequest`, and gossips it on a peer-credit topic. The state machine here
//! is the deterministic-side scaffolding; the consumer product plugs in the signing function
//! and the gossip transport.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BustState {
    Healthy,
    Warning,
    Busted,
    LoanRequested,
    LoanGranted,
    Recovered,
}

#[derive(Debug, Clone)]
pub struct BustDetector {
    pub warning_threshold: u128,
    pub bust_threshold: u128,
    state: BustState,
}

impl BustDetector {
    pub fn new(warning_threshold: u128, bust_threshold: u128) -> Self {
        assert!(bust_threshold <= warning_threshold);
        Self {
            warning_threshold,
            bust_threshold,
            state: BustState::Healthy,
        }
    }

    pub fn state(&self) -> BustState {
        self.state
    }

    /// Update the state machine on a new principal-balance observation. Returns the new state
    /// (which may equal the prior state if no transition fired).
    pub fn observe(&mut self, principal: u128) -> BustState {
        self.state = match (self.state, principal) {
            (_, p) if p <= self.bust_threshold => match self.state {
                BustState::LoanRequested | BustState::LoanGranted => self.state,
                _ => BustState::Busted,
            },
            (BustState::Busted, p) if p > self.bust_threshold && p <= self.warning_threshold => {
                BustState::Warning
            }
            (BustState::Recovered, _) => BustState::Recovered,
            (_, p) if p <= self.warning_threshold => BustState::Warning,
            (_, _) => BustState::Healthy,
        };
        self.state
    }

    /// Mark a loan request as sent. State machine transitions to `LoanRequested` if we're
    /// `Busted`; no-op otherwise.
    pub fn mark_loan_requested(&mut self) {
        if self.state == BustState::Busted {
            self.state = BustState::LoanRequested;
        }
    }

    /// Mark a loan grant as observed.
    pub fn mark_loan_granted(&mut self) {
        if matches!(self.state, BustState::LoanRequested | BustState::Busted) {
            self.state = BustState::LoanGranted;
        }
    }

    /// Mark a recovery (principal restored above warning threshold and loan repaid).
    pub fn mark_recovered(&mut self) {
        self.state = BustState::Recovered;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_above_warning() {
        let mut d = BustDetector::new(100, 20);
        assert_eq!(d.observe(150), BustState::Healthy);
    }

    #[test]
    fn warning_in_band() {
        let mut d = BustDetector::new(100, 20);
        assert_eq!(d.observe(80), BustState::Warning);
    }

    #[test]
    fn bust_below_threshold() {
        let mut d = BustDetector::new(100, 20);
        d.observe(80);
        assert_eq!(d.observe(15), BustState::Busted);
    }

    #[test]
    fn loan_lifecycle() {
        let mut d = BustDetector::new(100, 20);
        d.observe(150);
        d.observe(15);
        assert_eq!(d.state(), BustState::Busted);
        d.mark_loan_requested();
        assert_eq!(d.state(), BustState::LoanRequested);
        // Even a fresh low observation doesn't re-bust us; we're in flight.
        d.observe(10);
        assert_eq!(d.state(), BustState::LoanRequested);
        d.mark_loan_granted();
        assert_eq!(d.state(), BustState::LoanGranted);
        d.mark_recovered();
        assert_eq!(d.state(), BustState::Recovered);
    }
}
