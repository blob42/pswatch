#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use mock_instant::thread_local::Instant;



/// a resource matcher for a given condition
pub trait ConditionMatcher {
    type Condition;

    /// Fully matches the condition
    fn matches(&self, c: Self::Condition) -> bool;

    /// Partial match of condition
    /// return None if implementer does not want to handle partial matching
    fn partial_match(&self, c: Self::Condition) -> Option<bool>;
}

pub trait StateTracker {
    type State;

    fn state(&self) -> Self::State;

    fn prev_state(&self) -> Option<Self::State>;

    /// whether we are exiting a state
    /// Example a Seen process becomes NotSeen
    fn exiting(&self) -> bool;

    fn update_state(&mut self, info: &sysinfo::System, t_refresh: Instant) -> Self::State;

}
