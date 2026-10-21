//! Contracts of node p0-api (wave 3): typed refusals and `#[non_exhaustive]`.
//!
//! One test per [`Unsupported`] variant, the `Display` text of each (equal to
//! the message of the `String` refusal it replaces), and a golden JSON of a
//! Robertson run's `Stats` committed from the tip before the change.

use vigilode::{
    Atol, CallbackError, Direction, InteriorError, Jacobian, Options, Problem, Rodas5p, SolveError,
    Unsupported,
};

const SPAN: (f64, f64) = (0.0, 40.0);
const Y0: [f64; 3] = [1.0, 0.0, 0.0];

fn rhs(_t: f64, y: &[f64], dy: &mut [f64]) -> Result<(), CallbackError> {
    dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
    dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
    dy[2] = 3.0e7 * y[1] * y[1];
    Ok(())
}

fn jac(_t: f64, y: &[f64], j: &mut [f64]) -> Result<(), CallbackError> {
    j.copy_from_slice(&[
        -0.04,
        1.0e4 * y[2],
        1.0e4 * y[1],
        0.04,
        -1.0e4 * y[2] - 6.0e7 * y[1],
        -1.0e4 * y[1],
        0.0,
        6.0e7 * y[1],
        0.0,
    ]);
    Ok(())
}

fn dense() -> Problem {
    Problem::new(3, rhs)
        .jacobian(Jacobian::dense(jac))
        .autonomous()
}

fn jvp() -> Problem {
    Problem::new(3, rhs)
        .jacobian(Jacobian::jvp(|_t, _y, v, out| {
            out.copy_from_slice(v);
            Ok(())
        }))
        .autonomous()
}

fn dense_output() -> Options {
    Options {
        dense_output: true,
        ..Options::default()
    }
}

fn vector() -> Options {
    Options {
        atol: Atol::Vector(vec![1.0e-10; 3]),
        ..Options::default()
    }
}

fn refusal(problem: &Problem, opts: &Options) -> Unsupported {
    match Rodas5p::solve(problem, SPAN, &Y0, opts) {
        Err(SolveError::Unsupported(u)) => u,
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

const MASS: [f64; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

#[test]
fn jvp_with_mass_or_dense_output() {
    let e = Rodas5p::solve(&jvp(), SPAN, &Y0, &dense_output()).unwrap_err();
    assert!(
        matches!(
            e,
            SolveError::Unsupported(Unsupported::JvpWithMassOrDenseOutput)
        ),
        "{e:?}"
    );
    let m = refusal(&jvp().mass(MASS.to_vec()), &Options::default());
    assert!(matches!(m, Unsupported::JvpWithMassOrDenseOutput));
}

#[test]
fn declared_band_on_dense_with_mass_or_dense_output() {
    let e = Rodas5p::solve(&dense().banded(1, 1), SPAN, &Y0, &dense_output()).unwrap_err();
    assert!(
        matches!(
            e,
            SolveError::Unsupported(Unsupported::DeclaredBandOnDenseWithMassOrDenseOutput)
        ),
        "{e:?}"
    );
}

#[test]
fn vector_atol_on_sequential_route() {
    let opts = Options {
        dense_output: true,
        interior_error: InteriorError::Enforce,
        ..vector()
    };
    let e = Rodas5p::solve(&dense(), SPAN, &Y0, &opts).unwrap_err();
    assert!(
        matches!(
            e,
            SolveError::Unsupported(Unsupported::VectorAtolOnSequentialRoute)
        ),
        "{e:?}"
    );
    let m = refusal(&dense().mass(MASS.to_vec()), &vector());
    assert!(matches!(m, Unsupported::VectorAtolOnSequentialRoute));
}

#[test]
fn vector_atol_on_matrix_free_route() {
    let e = Rodas5p::solve(&jvp(), SPAN, &Y0, &vector()).unwrap_err();
    assert!(
        matches!(
            e,
            SolveError::Unsupported(Unsupported::VectorAtolOnMatrixFreeRoute)
        ),
        "{e:?}"
    );
}

#[test]
fn vector_atol_with_declared_band_on_dense() {
    let e = Rodas5p::solve(&dense().banded(2, 2), SPAN, &Y0, &vector()).unwrap_err();
    assert!(
        matches!(
            e,
            SolveError::Unsupported(Unsupported::VectorAtolWithDeclaredBandOnDense)
        ),
        "{e:?}"
    );
}

#[test]
fn events_without_dense_output() {
    let p = dense().event(|_t, y| y[0] - 0.5, Direction::Down);
    let e = Rodas5p::solve(&p, SPAN, &Y0, &Options::default()).unwrap_err();
    assert!(
        matches!(
            e,
            SolveError::Unsupported(Unsupported::EventsWithoutDenseOutput)
        ),
        "{e:?}"
    );
}

#[test]
fn band_declaration_on_jvp_problem() {
    let e = Rodas5p::solve(&jvp().banded(1, 1), SPAN, &Y0, &Options::default()).unwrap_err();
    assert!(
        matches!(
            e,
            SolveError::Unsupported(Unsupported::BandDeclarationOnJvpProblem)
        ),
        "{e:?}"
    );
}

/// `Display` equals the message of the former `String` refusal.
#[test]
fn display_text_is_the_former_message() {
    let text = |u: Unsupported| u.to_string();
    assert_eq!(
        text(refusal(&jvp(), &dense_output())),
        "a mass matrix or dense output on a JVP-only problem: the protected sequential \
         dense driver needs an explicit or finite-difference Jacobian (the step's stage \
         matrix M - h gamma J)"
    );
    assert_eq!(
        text(refusal(&dense().banded(1, 1), &dense_output())),
        "a declared band on a dense Jacobian with a mass matrix or dense output: only the \
         router verifies such a declaration, and the sequential dense driver would ignore it"
    );
    assert_eq!(
        text(refusal(&dense().mass(MASS.to_vec()), &vector())),
        "a vector atol with a mass matrix or interior_error = Enforce: the \
         sequential dense driver takes a scalar absolute tolerance"
    );
    assert_eq!(
        text(refusal(&jvp(), &vector())),
        "a vector atol on a JVP-only problem: the matrix-free driver's Krylov \
         weights take a scalar absolute tolerance"
    );
    assert_eq!(
        text(refusal(&dense().banded(2, 2), &vector())),
        "a vector atol on a dense Jacobian with a declared band: the router that \
         verifies the declaration takes a scalar absolute tolerance"
    );
    let p = dense().event(|_t, y| y[0] - 0.5, Direction::Down);
    assert_eq!(
        text(refusal(&p, &Options::default())),
        "events without dense output: events are located on the continuous extension of \
         the accepted steps"
    );
    assert_eq!(
        text(refusal(&jvp().banded(1, 1), &Options::default())),
        "a band declaration on a JVP-only problem"
    );
    let e = Rodas5p::solve(&jvp().banded(1, 1), SPAN, &Y0, &Options::default()).unwrap_err();
    assert_eq!(
        e.to_string(),
        "unsupported: a band declaration on a JVP-only problem"
    );
}

/// The serde JSON of a Robertson run's `Stats` is byte-identical to the one
/// committed from the tip before p0-api (`tests/golden/`).
#[test]
fn stats_json_unchanged() {
    let sol = Rodas5p::solve(&dense(), SPAN, &Y0, &Options::default()).unwrap();
    let json = serde_json::to_string_pretty(&sol.stats).unwrap();
    let golden = include_str!("golden/robertson_stats.json");
    assert_eq!(json, golden.trim_end_matches('\n'));
}
