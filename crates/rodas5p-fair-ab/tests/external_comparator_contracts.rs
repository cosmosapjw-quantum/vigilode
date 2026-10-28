use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use rodas5p_core::sha256_hex;
use rodas5p_fair_ab::{
    EXTERNAL_COMPARATOR_EVIDENCE_SCHEMA_VERSION, ExternalComparatorContract,
    ExternalComparatorEvidence, ExternalComparatorKind, ExternalDenseOutputPolicy,
    ExternalEvidenceChecksums, ExternalMassTreatment, ExternalNativeWork, ExternalProblemBinding,
    ExternalReferenceDependency, ExternalRunStatus, ExternalRunnerBinding,
    ExternalRunnerDependency, ExternalRuntimeIdentity, ExternalToleranceBinding,
    SundialsProbeFinding, external_runner_dependency_closure_checksum,
    external_runtime_identity_checksum, load_external_comparator_evidence,
    numerical_reference_grid_checksum, numerical_reference_state_checksum,
    numerical_reference_v2_not_run_manifest, sundials_probe_evidence_checksum,
};

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn dependency_closure() -> String {
    external_runner_dependency_closure_checksum(&[
        ExternalRunnerDependency {
            path: "fixtures/oracle.json".into(),
            sha256: sha('a'),
        },
        ExternalRunnerDependency {
            path: "tools/external.py".into(),
            sha256: sha('b'),
        },
    ])
    .unwrap()
}

fn sundials_runtime(
    cvode_available: bool,
    ida_only_version: Option<String>,
) -> ExternalRuntimeIdentity {
    let executable_names_checked: Vec<String> = vec!["cvode".into(), "cvode_*".into()];
    let pkg_config_modules_checked = vec!["sundials-cvode".into()];
    let header_paths_checked = vec!["/usr/include/cvode/cvode.h".into()];
    let library_names_checked = vec!["libsundials_cvode.so".into()];
    let python_modules_checked = vec!["scikits.odes".into(), "sundials".into()];
    let probe_findings = [
        ("executable", &executable_names_checked),
        ("pkg-config", &pkg_config_modules_checked),
        ("header", &header_paths_checked),
        ("library", &library_names_checked),
        ("python-module", &python_modules_checked),
    ]
    .into_iter()
    .flat_map(|(category, targets)| {
        targets.iter().map(move |target| SundialsProbeFinding {
            category: category.into(),
            target: target.clone(),
            observed: cvode_available,
            detail: if cvode_available {
                format!("observed {target}")
            } else {
                "not found".into()
            },
        })
    })
    .collect::<Vec<_>>();
    let probe_evidence_sha256 = sundials_probe_evidence_checksum(
        cvode_available,
        &executable_names_checked,
        &pkg_config_modules_checked,
        &header_paths_checked,
        &library_names_checked,
        &python_modules_checked,
        &ida_only_version,
        &probe_findings,
    )
    .unwrap();
    ExternalRuntimeIdentity::SundialsHostProbe {
        cvode_available,
        executable_names_checked,
        pkg_config_modules_checked,
        header_paths_checked,
        library_names_checked,
        python_modules_checked,
        ida_only_version,
        probe_findings,
        probe_evidence_sha256,
    }
}

fn refresh_sundials_checksums(runtime: &mut ExternalRuntimeIdentity) -> String {
    let ExternalRuntimeIdentity::SundialsHostProbe {
        cvode_available,
        executable_names_checked,
        pkg_config_modules_checked,
        header_paths_checked,
        library_names_checked,
        python_modules_checked,
        ida_only_version,
        probe_findings,
        probe_evidence_sha256,
    } = runtime
    else {
        panic!("expected SUNDIALS runtime");
    };
    *probe_evidence_sha256 = sundials_probe_evidence_checksum(
        *cvode_available,
        executable_names_checked,
        pkg_config_modules_checked,
        header_paths_checked,
        library_names_checked,
        python_modules_checked,
        ida_only_version,
        probe_findings,
    )
    .unwrap();
    external_runtime_identity_checksum(runtime).unwrap()
}

struct EvidenceFile(PathBuf);

impl EvidenceFile {
    fn write(evidence: &ExternalComparatorEvidence) -> Self {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vigilode-external-comparator-{}-{sequence}.json",
            std::process::id()
        ));
        fs::write(&path, serde_json::to_vec(evidence).unwrap()).unwrap();
        Self(path)
    }

    fn write_json(value: &serde_json::Value) -> Self {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vigilode-external-comparator-{}-{sequence}.json",
            std::process::id()
        ));
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        Self(path)
    }
}

impl Drop for EvidenceFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn scipy_fixture() -> (ExternalComparatorContract, ExternalComparatorEvidence) {
    let times = vec![0.0, 0.5, 1.0];
    let states = vec![
        vec![1.0],
        vec![0.606_530_659_712_633_4],
        vec![0.367_879_441_171_442_33],
    ];
    let lineage = "scipy-radau@8c75ae75176236f233824e9a0483c26a69e6dfec".to_owned();
    let runtime = ExternalRuntimeIdentity::ScipyPython {
        identity: numerical_reference_v2_not_run_manifest().unwrap().runtime,
    };
    let runner = ExternalRunnerBinding {
        runner_id: "scipy-solve-ivp-radau".into(),
        version: "1.17.0".into(),
        build_id: "cp312-manylinux-x86_64".into(),
        implementation_lineage_id: lineage.clone(),
        script_path: "tools/external_comparators/scipy_radau.py".into(),
        script_sha256: sha('1'),
        dependency_closure_sha256: dependency_closure(),
        source_repository: "https://github.com/scipy/scipy".into(),
        source_revision: "8c75ae75176236f233824e9a0483c26a69e6dfec".into(),
        source_sha256: "d0aa4593431ef39ee07825db6ef0324e4a9bacef0e23fda42d377318ba6a6256".into(),
        observed_upstream_identity: true,
        runtime_identity_sha256: external_runtime_identity_checksum(&runtime).unwrap(),
        runtime,
    };
    let problem = ExternalProblemBinding {
        case_id: "scalar-linear-contract-rtol-1e-8".into(),
        problem_id: "scalar-linear-contract".into(),
        implementation_revision: "1".repeat(40),
        dimension: 1,
        t_span: [0.0, 1.0],
        problem_source_sha256: sha('3'),
        has_mass_matrix: false,
        requested_times: times.clone(),
        output_grid_id: "scalar-grid-v1".into(),
        reference_checksum: sha('4'),
    };
    let tolerance = ExternalToleranceBinding {
        rtol: 1.0e-8,
        atol: 1.0e-10,
    };
    let dense_output = ExternalDenseOutputPolicy {
        interpolation: "scipy-radau-cubic-collocation".into(),
        solver_dense_output: true,
        controller_step_clipping: false,
    };
    let contract = ExternalComparatorContract {
        comparator: ExternalComparatorKind::ScipyRadau,
        runner: runner.clone(),
        problem: problem.clone(),
        tolerance: tolerance.clone(),
        dense_output: dense_output.clone(),
        mass_treatment: ExternalMassTreatment::Identity,
        reference_lineage_id: lineage.clone(),
    };
    let evidence = ExternalComparatorEvidence {
        schema_version: EXTERNAL_COMPARATOR_EVIDENCE_SCHEMA_VERSION.into(),
        comparator: ExternalComparatorKind::ScipyRadau,
        runner,
        problem,
        tolerance,
        dense_output,
        mass_treatment: ExternalMassTreatment::Identity,
        reference_dependency: ExternalReferenceDependency {
            reference_lineage_id: lineage.clone(),
            runner_lineage_id: lineage,
            shares_implementation_lineage: true,
        },
        status: ExternalRunStatus::Success,
        checksums: ExternalEvidenceChecksums {
            grid_sha256: numerical_reference_grid_checksum(&times),
            committed_grid_sha256: Some(numerical_reference_grid_checksum(&times)),
            state_sha256: Some(numerical_reference_state_checksum(&states)),
        },
        committed_times: Some(times),
        states: Some(states),
        native_work: Some(ExternalNativeWork::ScipyRadau {
            nfev: 123,
            njev: 4,
            nlu: 12,
        }),
    };
    (contract, evidence)
}

fn sundials_fixture() -> (ExternalComparatorContract, ExternalComparatorEvidence) {
    let (mut contract, mut evidence) = scipy_fixture();
    let runtime = sundials_runtime(true, None);
    let runner = ExternalRunnerBinding {
        runner_id: "sundials-cvode".into(),
        version: "7.4.0".into(),
        build_id: "double-precision-openmp-off-klu-on".into(),
        implementation_lineage_id: "sundials-cvode@7.4.0+double-klu".into(),
        script_path: "tools/external_comparators/sundials_cvode_runner".into(),
        script_sha256: sha('5'),
        dependency_closure_sha256: dependency_closure(),
        source_repository: "https://github.com/LLNL/sundials".into(),
        source_revision: "v7.4.0".into(),
        source_sha256: sha('6'),
        observed_upstream_identity: true,
        runtime_identity_sha256: external_runtime_identity_checksum(&runtime).unwrap(),
        runtime,
    };
    contract.comparator = ExternalComparatorKind::SundialsCvode;
    contract.runner = runner.clone();
    evidence.comparator = ExternalComparatorKind::SundialsCvode;
    evidence.runner = runner.clone();
    evidence.reference_dependency.runner_lineage_id = runner.implementation_lineage_id;
    evidence.reference_dependency.shares_implementation_lineage = false;
    evidence.native_work = Some(ExternalNativeWork::SundialsCvode {
        nst: 40,
        nfe: 91,
        nje: 3,
        nni: 50,
        ncfn: 0,
        netf: 2,
        nli: 63,
        nsetups: 7,
    });
    (contract, evidence)
}

fn assert_rejected(contract: &ExternalComparatorContract, evidence: &ExternalComparatorEvidence) {
    let file = EvidenceFile::write(evidence);
    assert!(load_external_comparator_evidence(&file.0, contract).is_err());
}

#[test]
fn valid_scipy_evidence_is_loaded_without_executing_the_runner() {
    let (contract, evidence) = scipy_fixture();
    let file = EvidenceFile::write(&evidence);
    let loaded = load_external_comparator_evidence(&file.0, &contract).unwrap();

    assert_eq!(loaded, evidence);
    assert!(loaded.reference_dependency.shares_implementation_lineage);
    assert!(matches!(
        loaded.native_work,
        Some(ExternalNativeWork::ScipyRadau {
            nfev: 123,
            njev: 4,
            nlu: 12
        })
    ));
}

#[test]
fn valid_sundials_success_preserves_cvode_native_work_without_rust_counter_mapping() {
    let (contract, evidence) = sundials_fixture();
    let file = EvidenceFile::write(&evidence);
    let loaded = load_external_comparator_evidence(&file.0, &contract).unwrap();

    assert!(!loaded.reference_dependency.shares_implementation_lineage);
    assert!(matches!(
        loaded.native_work,
        Some(ExternalNativeWork::SundialsCvode {
            nst: 40,
            nfe: 91,
            nje: 3,
            nni: 50,
            ncfn: 0,
            netf: 2,
            nli: 63,
            nsetups: 7
        })
    ));
}

#[test]
fn every_runner_problem_policy_checksum_and_lineage_mutation_is_rejected() {
    let (contract, evidence) = scipy_fixture();

    let mut mutated = evidence.clone();
    mutated.schema_version = "unknown-schema".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.comparator = ExternalComparatorKind::SundialsCvode;
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.runner.runner_id = "other-runner".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.runner.version = "1.16.0".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.runner.build_id = "another-wheel".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.runner.script_sha256 = sha('7');
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.runner.dependency_closure_sha256 = sha('d');
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.runner.script_path = "another-script.py".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.runner.implementation_lineage_id = "other-lineage".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.runner.source_repository = "https://example.invalid/scipy".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.runner.source_revision = "wrong-revision".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.runner.source_sha256 = sha('c');
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.problem.case_id = "another-case".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.problem.problem_id = "another-problem".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.problem.implementation_revision = "2".repeat(40);
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.problem.dimension = 2;
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.problem.t_span[1] = 2.0;
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.problem.problem_source_sha256 = sha('8');
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.problem.has_mass_matrix = true;
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.problem.requested_times[1] = 0.25;
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.problem.output_grid_id = "another-grid".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.problem.reference_checksum = sha('9');
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.tolerance.rtol = 2.0e-8;
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.tolerance.atol = 2.0e-10;
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.dense_output.interpolation = "another-interpolant".into();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.dense_output.solver_dense_output = false;
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.dense_output.controller_step_clipping = true;
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.checksums.grid_sha256 = sha('a');
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.states.as_mut().unwrap()[1][0] = 0.5;
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.checksums.state_sha256 = Some(sha('d'));
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.states.as_mut().unwrap().pop();
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.native_work = Some(ExternalNativeWork::SundialsCvode {
        nst: 1,
        nfe: 1,
        nje: 1,
        nni: 1,
        ncfn: 0,
        netf: 0,
        nli: 1,
        nsetups: 1,
    });
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.reference_dependency.shares_implementation_lineage = false;
    assert_rejected(&contract, &mutated);
    let mut mutated = evidence.clone();
    mutated.reference_dependency.reference_lineage_id = "independent-reference".into();
    assert_rejected(&contract, &mutated);

    let mut unknown = serde_json::to_value(&evidence).unwrap();
    unknown["fallback_runner"] = serde_json::json!("internal-radau");
    let file = EvidenceFile::write_json(&unknown);
    assert!(load_external_comparator_evidence(&file.0, &contract).is_err());
}

#[test]
fn external_runner_dependency_closure_is_ordered_domain_separated_and_golden() {
    let ordered = vec![
        ExternalRunnerDependency {
            path: "a".into(),
            sha256: sha('1'),
        },
        ExternalRunnerDependency {
            path: "z/path".into(),
            sha256: sha('f'),
        },
    ];
    assert_eq!(
        external_runner_dependency_closure_checksum(&ordered).unwrap(),
        "d499257749d200567795121ec7bca25b92b53745d0d41a0632866a8ed05f54e1"
    );
    let mut unsorted = ordered.clone();
    unsorted.reverse();
    assert!(external_runner_dependency_closure_checksum(&unsorted).is_err());
    let duplicate = vec![ordered[0].clone(), ordered[0].clone()];
    assert!(external_runner_dependency_closure_checksum(&duplicate).is_err());
    let mut traversal = ordered;
    traversal[0].path = "../a".into();
    assert!(external_runner_dependency_closure_checksum(&traversal).is_err());
}

#[test]
fn python_generated_scipy_and_sundials_fixtures_cross_the_strict_rust_loader() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dependency_paths = [
        "fixtures/scientific_corpus_v2_1_calibration_oracle.json",
        "fixtures/scientific_corpus_v2_1_semilinear_oracle.json",
        "tools/reference_v2/generate_references.py",
        "tools/reference_v2/generate_references_v2.py",
        "tools/scientific_validity_v2/external_evidence.py",
    ];
    let dependencies = dependency_paths
        .iter()
        .map(|path| ExternalRunnerDependency {
            path: (*path).into(),
            sha256: sha256_hex(&fs::read(repository.join(path)).unwrap()),
        })
        .collect::<Vec<_>>();
    let closure = external_runner_dependency_closure_checksum(&dependencies).unwrap();
    assert_eq!(
        closure,
        "bde3b015c2ec18b4058fa15a88b138baf3edb6d913a6df38b6b1d1cb2ae5e40c"
    );
    assert_eq!(
        dependencies.last().unwrap().sha256,
        "f3124f15505dafde3995ed9b80a8ee5fc056e24621d96d0ec2fef3565766e10a"
    );

    for (name, raw_sha, comparator) in [
        (
            "scipy_radau_success.json",
            "297b7745963d36523df260c2e238aef6ab8ed29ca1092433e9e2d9db606642a2",
            ExternalComparatorKind::ScipyRadau,
        ),
        (
            "sundials_cvode_unavailable.json",
            "7fcf76c055e3b3e34f44fc26e295309b16f0ffd94c6e767393df094e9f826001",
            ExternalComparatorKind::SundialsCvode,
        ),
    ] {
        let path = repository
            .join("tools/scientific_validity_v2/fixtures")
            .join(name);
        let bytes = fs::read(&path).unwrap();
        assert_eq!(sha256_hex(&bytes), raw_sha);
        let evidence: ExternalComparatorEvidence = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(evidence.comparator, comparator);
        assert_eq!(evidence.runner.dependency_closure_sha256, closure);
        let contract = ExternalComparatorContract {
            comparator,
            runner: evidence.runner.clone(),
            problem: evidence.problem.clone(),
            tolerance: evidence.tolerance.clone(),
            dense_output: evidence.dense_output.clone(),
            mass_treatment: evidence.mass_treatment.clone(),
            reference_lineage_id: evidence.reference_dependency.reference_lineage_id.clone(),
        };
        assert_eq!(
            load_external_comparator_evidence(&path, &contract).unwrap(),
            evidence
        );
    }
}

#[test]
fn unavailable_and_not_run_sundials_are_typed_and_cannot_carry_substitute_work() {
    for status in [
        ExternalRunStatus::Unavailable {
            reason: "SUNDIALS runtime is not installed".into(),
        },
        ExternalRunStatus::NotRun {
            reason: "external execution was not authorized".into(),
        },
    ] {
        let (mut contract, mut evidence) = sundials_fixture();
        if matches!(status, ExternalRunStatus::Unavailable { .. }) {
            let runtime = sundials_runtime(false, Some("6.4.1".into()));
            let runtime_sha = external_runtime_identity_checksum(&runtime).unwrap();
            contract.runner.runtime = runtime.clone();
            contract.runner.runtime_identity_sha256 = runtime_sha.clone();
            contract.runner.version = "not-installed".into();
            contract.runner.build_id = "not-installed".into();
            contract.runner.source_revision = "not-observed".into();
            contract.runner.source_sha256 = "0".repeat(64);
            contract.runner.observed_upstream_identity = false;
            evidence.runner.runtime = runtime;
            evidence.runner.runtime_identity_sha256 = runtime_sha;
            evidence.runner.version = "not-installed".into();
            evidence.runner.build_id = "not-installed".into();
            evidence.runner.source_revision = "not-observed".into();
            evidence.runner.source_sha256 = "0".repeat(64);
            evidence.runner.observed_upstream_identity = false;
        }
        evidence.status = status;
        evidence.states = None;
        evidence.committed_times = None;
        evidence.checksums.committed_grid_sha256 = None;
        evidence.checksums.state_sha256 = None;
        evidence.native_work = None;
        let file = EvidenceFile::write(&evidence);
        let loaded = load_external_comparator_evidence(&file.0, &contract).unwrap();
        assert!(matches!(
            loaded.status,
            ExternalRunStatus::Unavailable { .. } | ExternalRunStatus::NotRun { .. }
        ));

        let mut substituted = evidence.clone();
        substituted.native_work = Some(ExternalNativeWork::ScipyRadau {
            nfev: 1,
            njev: 1,
            nlu: 1,
        });
        assert_rejected(&contract, &substituted);
        let mut fabricated_states = evidence.clone();
        fabricated_states.states = Some(vec![vec![1.0], vec![0.5], vec![0.25]]);
        fabricated_states.committed_times = Some(vec![0.0, 0.5, 1.0]);
        fabricated_states.checksums.committed_grid_sha256 = Some(
            numerical_reference_grid_checksum(fabricated_states.committed_times.as_ref().unwrap()),
        );
        fabricated_states.checksums.state_sha256 = Some(numerical_reference_state_checksum(
            fabricated_states.states.as_ref().unwrap(),
        ));
        assert_rejected(&contract, &fabricated_states);
        let mut wrong_kind = evidence.clone();
        wrong_kind.comparator = ExternalComparatorKind::ScipyRadau;
        assert_rejected(&contract, &wrong_kind);
    }
}

#[test]
fn sundials_probe_requires_exact_one_to_one_target_findings() {
    for duplicate in [false, true] {
        let (mut contract, mut evidence) = sundials_fixture();
        let mut runtime = contract.runner.runtime.clone();
        let ExternalRuntimeIdentity::SundialsHostProbe { probe_findings, .. } = &mut runtime else {
            unreachable!()
        };
        if duplicate {
            probe_findings.push(probe_findings[0].clone());
        } else {
            probe_findings.pop();
        }
        let runtime_sha = refresh_sundials_checksums(&mut runtime);
        contract.runner.runtime = runtime.clone();
        contract.runner.runtime_identity_sha256 = runtime_sha.clone();
        evidence.runner.runtime = runtime;
        evidence.runner.runtime_identity_sha256 = runtime_sha;
        assert_rejected(&contract, &evidence);
    }
}

#[test]
fn scipy_solver_failure_preserves_native_work_and_an_authenticated_prefix() {
    let (contract, mut evidence) = scipy_fixture();
    evidence.status = ExternalRunStatus::SolverFailure {
        reason: "solver stopped after returning two requested outputs".into(),
    };
    let times = evidence.committed_times.as_mut().unwrap();
    times.pop();
    let states = evidence.states.as_mut().unwrap();
    states.pop();
    evidence.checksums.committed_grid_sha256 = Some(numerical_reference_grid_checksum(times));
    evidence.checksums.state_sha256 = Some(numerical_reference_state_checksum(states));
    let file = EvidenceFile::write(&evidence);
    assert_eq!(
        load_external_comparator_evidence(&file.0, &contract).unwrap(),
        evidence
    );

    let mut bad_prefix = evidence.clone();
    bad_prefix.committed_times.as_mut().unwrap()[1] = 0.25;
    bad_prefix.checksums.committed_grid_sha256 = Some(numerical_reference_grid_checksum(
        bad_prefix.committed_times.as_ref().unwrap(),
    ));
    assert_rejected(&contract, &bad_prefix);

    let mut bad_work = evidence.clone();
    bad_work.native_work = Some(ExternalNativeWork::SundialsCvode {
        nst: 1,
        nfe: 1,
        nje: 1,
        nni: 1,
        ncfn: 0,
        netf: 0,
        nli: 1,
        nsetups: 1,
    });
    assert_rejected(&contract, &bad_work);

    let mut no_prefix = evidence;
    no_prefix.committed_times = None;
    no_prefix.states = None;
    no_prefix.checksums.committed_grid_sha256 = None;
    no_prefix.checksums.state_sha256 = None;
    let file = EvidenceFile::write(&no_prefix);
    assert!(load_external_comparator_evidence(&file.0, &contract).is_ok());

    let mut no_work = no_prefix;
    no_work.native_work = None;
    assert_rejected(&contract, &no_work);
}

#[test]
fn mass_cases_fail_closed_unless_a_transformed_identity_is_pinned() {
    let (mut identity_contract, mut identity) = scipy_fixture();
    identity_contract.problem.has_mass_matrix = true;
    identity.problem.has_mass_matrix = true;
    assert_rejected(&identity_contract, &identity);

    let (mut nonapp_contract, mut nonapp) = scipy_fixture();
    nonapp_contract.problem.has_mass_matrix = true;
    nonapp.problem.has_mass_matrix = true;
    nonapp_contract.mass_treatment = ExternalMassTreatment::NonApplicable;
    nonapp.mass_treatment = ExternalMassTreatment::NonApplicable;
    nonapp.status = ExternalRunStatus::NonApplicable {
        reason: "SciPy Radau has no native mass-matrix contract".into(),
    };
    nonapp.states = None;
    nonapp.committed_times = None;
    nonapp.checksums.committed_grid_sha256 = None;
    nonapp.checksums.state_sha256 = None;
    nonapp.native_work = None;
    let file = EvidenceFile::write(&nonapp);
    assert!(load_external_comparator_evidence(&file.0, &nonapp_contract).is_ok());

    let mut wrong_status = nonapp.clone();
    wrong_status.status = ExternalRunStatus::NotRun {
        reason: "wrong status for non-applicable mass".into(),
    };
    assert_rejected(&nonapp_contract, &wrong_status);

    let (mut transformed_contract, mut transformed) = scipy_fixture();
    transformed_contract.problem.has_mass_matrix = true;
    transformed.problem.has_mass_matrix = true;
    let treatment = ExternalMassTreatment::TransformedIdentity {
        transform_id: "constant-mass-lu-transform-v1".into(),
        transform_source_sha256: sha('b'),
    };
    transformed_contract.mass_treatment = treatment.clone();
    transformed.mass_treatment = treatment;
    let file = EvidenceFile::write(&transformed);
    assert!(load_external_comparator_evidence(&file.0, &transformed_contract).is_ok());

    if let ExternalMassTreatment::TransformedIdentity { transform_id, .. } =
        &mut transformed_contract.mass_treatment
    {
        transform_id.clear();
    }
    if let ExternalMassTreatment::TransformedIdentity { transform_id, .. } =
        &mut transformed.mass_treatment
    {
        transform_id.clear();
    }
    assert_rejected(&transformed_contract, &transformed);
}

#[test]
fn a_missing_evidence_file_is_not_synthesized_into_an_external_run() {
    let (contract, _) = sundials_fixture();
    let missing = std::env::temp_dir().join(format!(
        "vigilode-missing-external-evidence-{}-{}.json",
        std::process::id(),
        FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    assert!(load_external_comparator_evidence(missing, &contract).is_err());
}
