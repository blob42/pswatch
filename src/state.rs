
#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use mock_instant::thread_local::Instant;

pub trait StateMatcher {
    type Condition;
    type State;

    fn matches(&self, c: Self::Condition) -> bool;
    fn state(&self) -> Self::State; 
    fn prev_state(&self) -> Option<Self::State>;

    // state is exiting
    fn exiting(&self) -> Option<Self::State> {
        self.prev_state().filter(|_s|{ ! matches!(self.state(), _s) })
    }
}

pub trait StateTracker {
    fn update_state(&mut self, info: &sysinfo::System, t_refresh: Instant) -> impl StateMatcher;
}
