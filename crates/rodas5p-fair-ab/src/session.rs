use rodas5p_krylov::{GcrodrState, GcrodrWorkspace, GmresWorkspace, LgmresState, LgmresWorkspace};
use serde::{Deserialize, Serialize};

use crate::{RecycleLifetime, SolverKind};

#[derive(Clone, Debug)]
pub struct SolverSession {
    pub solver: SolverKind,
    pub operator_id: Option<String>,
    pub previous_solution: Option<Vec<f64>>,
    pub lgmres: LgmresState,
    pub gcrodr: GcrodrState,
    pub gmres_workspace: GmresWorkspace,
    pub lgmres_workspace: LgmresWorkspace,
    pub gcrodr_workspace: GcrodrWorkspace,
    pub certificate_output: Vec<f64>,
    pub certificate_residual: Vec<f64>,
    pub generation: u64,
}

impl SolverSession {
    pub fn new(solver: SolverKind) -> Self {
        Self {
            solver,
            operator_id: None,
            previous_solution: None,
            lgmres: LgmresState::default(),
            gcrodr: GcrodrState::default(),
            gmres_workspace: GmresWorkspace::default(),
            lgmres_workspace: LgmresWorkspace::default(),
            gcrodr_workspace: GcrodrWorkspace::default(),
            certificate_output: Vec::new(),
            certificate_residual: Vec::new(),
            generation: 0,
        }
    }

    pub fn clear_recycle_state(&mut self) {
        self.operator_id = None;
        self.previous_solution = None;
        self.lgmres = LgmresState::default();
        self.gcrodr = GcrodrState::default();
        self.generation = 0;
    }

    pub fn clear(&mut self) {
        self.clear_recycle_state();
    }

    pub fn workspace_capacity_f64(&self) -> usize {
        self.gmres_workspace.capacity_f64()
            + self.lgmres_workspace.capacity_f64()
            + self.gcrodr_workspace.capacity_f64()
            + self.certificate_output.capacity()
            + self.certificate_residual.capacity()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    pub step_index: usize,
    pub reason: String,
    pub generation_before: u64,
    pub generation_after: u64,
}

pub struct RecycleSessionManager {
    lifetime: RecycleLifetime,
    session: SolverSession,
    current_step: Option<usize>,
    pub reset_count: u64,
    pub transition_log: Vec<StateTransition>,
}

impl RecycleSessionManager {
    pub fn new(solver: SolverKind, lifetime: RecycleLifetime) -> Self {
        Self {
            lifetime,
            session: SolverSession::new(solver),
            current_step: None,
            reset_count: 0,
            transition_log: Vec::new(),
        }
    }

    pub fn acquire(&mut self, step_index: usize) -> &mut SolverSession {
        let reason = match self.lifetime {
            RecycleLifetime::Off => Some("off_before_case"),
            RecycleLifetime::Stage if self.current_step.is_some_and(|step| step != step_index) => {
                Some("stage_boundary")
            }
            _ => None,
        };
        if let Some(reason) = reason {
            self.reset(step_index, reason);
        }
        self.current_step = Some(step_index);
        &mut self.session
    }

    pub fn reset(&mut self, step_index: usize, reason: impl Into<String>) {
        let before = self.session.generation;
        self.session.clear_recycle_state();
        self.reset_count += 1;
        self.transition_log.push(StateTransition {
            step_index,
            reason: reason.into(),
            generation_before: before,
            generation_after: self.session.generation,
        });
    }
}
