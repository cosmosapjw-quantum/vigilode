use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GoalBudgetPlan {
    Fixed,
    StateReplay,
    SecantReplay,
    Predictive,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct GoalBudgetPolicy {
    pub absolute: f64,
    pub plan: GoalBudgetPlan,
}
pub struct GoalBudgetRun {
    pub run: Option<SlabReplayRun>,
}
#[allow(clippy::too_many_arguments)]
pub fn evolve_epoch_goal_budget(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    branch: &[[f64; 2]],
    epoch: EvolutionEpoch,
    opts: &Options,
    policy: EvolutionPolicy,
    route: EvolutionRoute,
    grid: &[f64],
    retention: RetainedOutputRoute,
    goal: &[f64],
    budget: GoalBudgetPolicy,
) -> CoreResult<GoalBudgetRun> {
    let _ = (
        src, x, z, branch, epoch, opts, policy, route, grid, retention, goal, budget,
    );
    Err(bad("goal budget replay not implemented"))
}
