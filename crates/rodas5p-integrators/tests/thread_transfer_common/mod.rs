//! Shared fixtures of the thread-transfer research tests
//! (`docs/reviews/thread_transfer_20261002/NEXT_DEVELOPMENT_DAG.json`).
#![allow(dead_code)] // each test target uses a subset

use rodas5p_core::{CoreResult, directed::Interval, rodas5p_coefficients};
use rodas5p_integrators::{
    InverseWitness, Q2CertificateSource, QuadraticModel, QuadraticStageProblem, StageCertificate,
    StageTarget,
};

pub const R4_DIMENSIONS: [usize; 5] = [1, 2, 4, 8, 16];
pub const ATOL: f64 = 1.0e-8;
pub const RTOL: f64 = 1.0e-6;
pub const INITIAL_RADIUS: f64 = 1.0e-3;

/// The diagonal quadratic fixture of `r4_studies::homotopy_cost_study`,
/// unchanged.
pub struct R4Fixture {
    pub n: usize,
    pub target: StageTarget,
    pub problem: QuadraticStageProblem,
    pub candidate: Vec<Vec<f64>>,
    pub y_hat: Vec<f64>,
    pub e_hat: Vec<f64>,
    pub witness: InverseWitness,
}

pub fn r4_fixture(n: usize) -> CoreResult<R4Fixture> {
    let coeffs = rodas5p_coefficients()?;
    let target = StageTarget::sequential(coeffs)?;
    let a = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| if i == j { -1.0 - i as f64 } else { 0.0 })
                .collect()
        })
        .collect::<Vec<Vec<f64>>>();
    let q = (0..n)
        .map(|i| -0.05 * (1 + i % 3) as f64)
        .collect::<Vec<_>>();
    let model = QuadraticModel::new(format!("diagonal-quadratic-{n}"), a, q)?;
    let y = (0..n).map(|i| 1.0 + 0.1 * i as f64).collect::<Vec<_>>();
    let h = 0.05;
    let problem = model.stage_problem(0.0, &y, h)?;
    let f = (0..n)
        .map(|i| {
            problem.jacobian[i][i] * y[i] - 2.0 * problem.q[i] * y[i] * y[i]
                + problem.q[i] * y[i] * y[i]
        })
        .collect::<Vec<_>>();
    let candidate = vec![f.iter().map(|x| h * x).collect::<Vec<_>>(); 8];
    let y_hat = y.iter().zip(&f).map(|(y, f)| y + h * f).collect::<Vec<_>>();
    let e_hat = vec![0.0; n];
    let witness = InverseWitness::diagonal(&problem, target.gamma)?;
    Ok(R4Fixture {
        n,
        target,
        problem,
        candidate,
        y_hat,
        e_hat,
        witness,
    })
}

const STAGE_INPUTS: &str = "fixtures/thread_transfer_r4_stage_inputs.json";
const ROOT_ORACLE: &str = "fixtures/thread_transfer_r4_root_oracle.json";

fn workspace_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn hex(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn hex_rows(rows: &[Vec<f64>]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| row.iter().copied().map(hex).collect())
        .collect()
}

/// The exact inputs of the R4 fixtures, for the exact-rational root oracle
/// `tools/thread_transfer_root_oracle.py`: problem and candidate bits, and
/// the target's `alpha` and native `Gamma` bits (its coupling is the exact
/// real `alpha_ij + Gamma_ij`).
pub fn stage_inputs() -> serde_json::Value {
    let coeffs = rodas5p_coefficients().unwrap();
    let fixtures = R4_DIMENSIONS
        .iter()
        .map(|&n| {
            let f = r4_fixture(n).unwrap();
            let s = f.target.stages();
            // The target's coupling is exactly alpha + Gamma.
            for i in 0..s {
                for j in 0..i {
                    let sum =
                        Interval::exact_sum(f.target.alpha_rows[i][j], coeffs.gamma_matrix[(i, j)])
                            .unwrap();
                    assert_eq!(sum, f.target.coupling_rows[i][j]);
                    assert_eq!(f.target.alpha_rows[i][j], coeffs.alpha[(i, j)]);
                }
            }
            serde_json::json!({
                "dimension": n,
                "h": hex(f.problem.h),
                "jacobian": hex_rows(&f.problem.jacobian),
                "y": f.problem.y.iter().copied().map(hex).collect::<Vec<_>>(),
                "q": f.problem.q.iter().copied().map(hex).collect::<Vec<_>>(),
                "candidate": hex_rows(&f.candidate),
                "gamma": hex(f.target.gamma),
                "alpha_rows": hex_rows(&f.target.alpha_rows),
                "gamma_rows_strict_lower": (0..s)
                    .map(|i| (0..i).map(|j| hex(coeffs.gamma_matrix[(i, j)])).collect::<Vec<_>>())
                    .collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "schema": "vigilode-thread-transfer-r4-stage-inputs-v1",
        "target_id": r4_fixture(1).unwrap().target.id,
        "fixtures": fixtures,
    })
}

/// Writes [`stage_inputs`] to the fixture path.
pub fn write_stage_inputs() {
    let path = workspace_path(STAGE_INPUTS);
    let text = serde_json::to_string_pretty(&stage_inputs()).unwrap() + "\n";
    std::fs::write(&path, text).unwrap();
    println!("wrote {}", path.display());
}

fn bits(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

/// One-ulp brackets `[down, up]` of the exact distances `|K_hat - K*|`
/// from the oracle, after checking that the oracle was computed from the
/// fixture bits in memory.
pub fn root_distances(n: usize) -> Vec<Vec<[f64; 2]>> {
    let text = std::fs::read_to_string(workspace_path(ROOT_ORACLE))
        .unwrap_or_else(|error| panic!("{ROOT_ORACLE}: {error}"));
    let oracle: serde_json::Value = serde_json::from_str(&text).unwrap();
    let inputs = stage_inputs();
    let index = R4_DIMENSIONS.iter().position(|&m| m == n).unwrap();
    assert_eq!(
        oracle["inputs"]["fixtures"][index], inputs["fixtures"][index],
        "the root oracle is stale: its inputs differ from the fixture bits"
    );
    oracle["fixtures"][index]["stage_distance"]
        .as_array()
        .unwrap()
        .iter()
        .map(|stage| {
            stage
                .as_array()
                .unwrap()
                .iter()
                .map(|pair| {
                    [
                        bits(pair[0].as_str().unwrap()),
                        bits(pair[1].as_str().unwrap()),
                    ]
                })
                .collect()
        })
        .collect()
}

/// `bound >= |K_hat - K*|` decided against the exact bracket: `bound >= up`,
/// or a degenerate bracket and `bound >= down`. Undecidable counts as no.
pub fn encloses_root(certificate: &StageCertificate, distances: &[Vec<[f64; 2]>]) -> bool {
    distances.iter().enumerate().all(|(i, stage)| {
        stage.iter().enumerate().all(|(a, [down, up])| {
            let bound = certificate.stage_bound[i][a];
            bound >= *up || (down == up && bound >= *down)
        })
    })
}

/// Writes `value` to the path in `variable`, if set; a relative path is
/// taken from the workspace root (tests run in the crate directory).
pub fn write_output(variable: &str, value: &serde_json::Value) {
    if let Ok(path) = std::env::var(variable) {
        let path = workspace_path(&path);
        let text = serde_json::to_string_pretty(value).expect("serialize results") + "\n";
        std::fs::write(&path, text)
            .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
        println!("wrote {}", path.display());
    }
}
