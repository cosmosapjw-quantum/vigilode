#!/usr/bin/env python3
"""WP9B: true OpenEvolve ProgramDatabase plus measured physical MCTS prior correction.

Runs original cooperative-cubic parameter IVPs, not toy/GA-only perturbations.
It never calls an LLM and never edits the original RHS/native integrators.
"""
from __future__ import annotations

import argparse
import ast
import hashlib
import json
import math
import random
import re
import subprocess
import sys
import types
from collections import defaultdict
from pathlib import Path

import numpy as np
from scipy.integrate import solve_ivp

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
SOURCE_RHS = ROOT / "crates/vigilode-coupled/src/coupled_inverse_be.rs"
SOURCE_FIXTURES = ROOT / "crates/vigilode-coupled/examples/coupled_achieved_wp.rs"
SOURCE_LOCKS = {
    "rhs": "d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482",
    "fixtures": "1bfc17ec2ed4b902cc06073807388f6e332e31d6",
}
VENDOR_ZIP_LOCK = "18ac7f1f7b9e1aca7c1539a768ea6cf4e9c2aee5452d52e5371faef34727ba91"
LABELS = (
    "asymmetric_2", "asymmetric_2_refined",
    "coupled_3_cubic", "linear_exchange_control",
)
AXES = (
    "EXCHANGE_C01_REL", "EXCHANGE_ALL_REL", "CUBIC_ALL_REL",
    "DECAY_ALL_REL", "FORCING_CONST",
)
H = 0.03
T_END = 0.15
N_SAMPLES = 65
HOLDOUT_H = 0.018
HOLDOUT_END = 0.20
RTOL = 2e-12
ATOL = 2e-14
CONSENSUS = 5e-10
ALPHA = 2.0
M0 = 0.25
EPSILON = 0.1
BETA_MCTS = 4.0
BETA_MUT = 6.0
SEED = 170117


def git_blob(data: bytes) -> str:
    payload = b"blob " + str(len(data)).encode() + b"\0" + data
    return hashlib.sha1(payload).hexdigest()


def verify_locks():
    for name, path in (("rhs", SOURCE_RHS), ("fixtures", SOURCE_FIXTURES)):
        observed = git_blob(path.read_bytes())
        assert observed == SOURCE_LOCKS[name], (name, observed)
    assert subprocess.check_output(
        ["git", "-C", str(ROOT), "rev-parse", "HEAD"],
        text=True,
    ).strip() != "", "missing git checkout"


def split_top_level(expression: str) -> list[str]:
    chunks, stack, start = [], [], 0
    pairs = {")": "(", "]": "[", "}": "{"}
    for i, ch in enumerate(expression):
        if ch in "([{":
            stack.append(ch)
        elif ch in ")]}":
            assert stack and stack[-1] == pairs[ch]
            stack.pop()
        elif ch == "," and not stack:
            item = expression[start:i].strip()
            if item:
                chunks.append(item)
            start = i + 1
    rest = expression[start:].strip()
    if rest:
        chunks.append(rest)
    assert not stack
    return chunks


def source_cases() -> dict:
    """Read original Rust Source::new fixtures, not manually substituted parameters."""
    data = SOURCE_FIXTURES.read_text()
    parsed = {}
    for label in LABELS:
        first = data.index('"' + label + '"')
        token = "Source::new("
        i = data.index(token, first) + len(token)
        depth, end = 1, None
        for j in range(i, len(data)):
            if data[j] == "(":
                depth += 1
            elif data[j] == ")":
                depth -= 1
                if depth == 0:
                    end = j
                    break
        assert end is not None, label
        args = split_top_level(data[i:end])
        assert len(args) == 4, (label, len(args))
        values = [ast.literal_eval(s.replace("vec!", "")) for s in args]
        k, g, c, y0 = [np.asarray(v, dtype=np.float64) for v in values]
        assert k.ndim == g.ndim == y0.ndim == 1
        assert c.shape == (len(k), len(k))
        assert len(k) == len(g) == len(y0)
        assert np.all(np.isfinite(k)) and np.all(k > 0)
        assert np.all(g >= 0) and np.all(c >= 0) and np.all(y0 > 0)
        assert np.all(np.diag(c) == 0)
        parsed[label] = {"k": k, "g": g, "c": c, "y0": y0}
    for key in ("k", "g", "c", "y0"):
        assert np.array_equal(parsed["asymmetric_2"][key],
                              parsed["asymmetric_2_refined"][key])
    return parsed


def matrices(p: dict, axis: str, theta: float):
    k, g, c, y0 = (p[name].copy() for name in ("k", "g", "c", "y0"))
    q = np.zeros_like(k)
    if axis == "EXCHANGE_C01_REL":
        c[0, 1] *= (1 + theta)
    elif axis == "EXCHANGE_ALL_REL":
        c *= 1 + theta
    elif axis == "CUBIC_ALL_REL":
        g *= 1 + theta
    elif axis == "DECAY_ALL_REL":
        k *= 1 + theta
    elif axis == "FORCING_CONST":
        q = theta * p["k"] * y0
    else:
        raise ValueError(axis)
    assert np.min(k) > 0 and np.min(g) >= 0 and np.min(c) >= 0
    Q = c.copy()
    Q[np.diag_indices_from(Q)] = -c.sum(axis=1)
    return k, g, c, y0, q, Q


def stationary(Q):
    n = Q.shape[0]
    A = Q.T.copy()
    A[-1, :] = 1.0
    rhs = np.zeros(n)
    rhs[-1] = 1.0
    pi = np.linalg.solve(A, rhs)
    assert np.min(pi) > 0 and abs(np.sum(pi) - 1) < 2e-12
    assert np.max(np.abs(pi @ Q)) < 2e-12
    return pi


def ode_rhs(y, k, g, Q, q):
    return Q @ y - k * y - g * y**3 + q


def solve_one(p, axis, theta, end, method, n_samples):
    k, g, _, y0, q, Q = matrices(p, axis, theta)
    pi = stationary(Q)
    times = np.linspace(0.0, end, n_samples)
    sol = solve_ivp(lambda t, y: ode_rhs(y, k, g, Q, q),
                    (0.0, end), y0, method=method, t_eval=times,
                    rtol=RTOL, atol=ATOL)
    assert sol.success and sol.y.shape == (len(k), n_samples), sol.message
    assert np.all(np.isfinite(sol.y))
    Y = sol.y.T
    W = Y @ pi
    return {
        "method": method, "theta": theta, "nfev": int(sol.nfev),
        "njev": int(sol.njev), "nlu": int(sol.nlu),
        "time": times.tolist(), "state": Y.tolist(),
        "W": W.tolist(), "pi": pi.tolist(),
        "min_state": float(np.min(Y)),
    }


def evaluate(p, cohort, axis, h=H, end=T_END, n_samples=N_SAMPLES):
    trajectories, work = [], 0
    for theta in (0.0, +h, -h):
        for method in ("Radau", "DOP853"):
            row = solve_one(p, axis, theta, end, method, n_samples)
            trajectories.append(row)
            work += row["nfev"]
    consensus = max(
        max(abs(a - b) for row_a, row_b in zip(
            np.asarray(trajectories[i]["state"]),
            np.asarray(trajectories[i + 1]["state"]))
            for a, b in zip(row_a, row_b))
        for i in (0, 2, 4)
    )
    nonnegative = min(row["min_state"] for row in trajectories) >= -1e-12
    valid = bool(consensus <= CONSENSUS and nonnegative)
    W0, Wp, Wm = [trajectories[i]["W"][-1] for i in (0, 2, 4)]
    s1 = abs(Wp - Wm) / (2 * h * max(abs(W0), 1e-12))
    s2 = abs(Wp + Wm - 2 * W0) / (h*h*max(abs(W0), 1e-12))
    reward = ((0.75*math.tanh(s1)+0.25*math.tanh(s2)) /
              (1 + 0.001*work)) if valid else 0.0
    assert math.isfinite(reward) and 0 <= reward <= 1
    return {
        "cohort": cohort, "axis": axis, "h": h, "horizon": end,
        "source_equivalence": "asymmetric_2" if "asymmetric_2" in cohort else cohort,
        "consensus_max": consensus, "nonnegative": nonnegative,
        "valid": valid, "W0": W0, "Wp": Wp, "Wm": Wm,
        "s1": s1, "s2": s2, "reward": reward,
        "total_nfev": work, "solver_integrations": len(trajectories),
        "trajectories": trajectories,
    }


def sensitivity(p, axis, end=HOLDOUT_END):
    """First-variation IVP + differentiated stationary left null vector."""
    k, g, c, y0, _, Q = matrices(p, axis, 0.0)
    pi = stationary(Q)
    Qp = np.zeros_like(Q)
    if axis == "EXCHANGE_C01_REL":
        Qp[0, 1] = c[0, 1]
        Qp[0, 0] = -c[0, 1]
    elif axis == "EXCHANGE_ALL_REL":
        Qp = Q.copy()
    Ap = Q.T.copy()
    Ap[-1, :] = 1.0
    bp = -Qp.T @ pi
    bp[-1] = 0.0
    pi_prime = np.linalg.solve(Ap, bp)
    assert abs(np.sum(pi_prime)) < 1e-11
    assert np.max(np.abs(pi_prime @ Q + pi @ Qp)) < 1e-10

    def f_theta(y):
        if axis in ("EXCHANGE_C01_REL", "EXCHANGE_ALL_REL"):
            return Qp @ y
        if axis == "CUBIC_ALL_REL":
            return -g*y**3
        if axis == "DECAY_ALL_REL":
            return -k*y
        return k*y0

    n = len(k)
    def augmented(t, yz):
        y, z = yz[:n], yz[n:]
        jac = Q - np.diag(k + 3*g*y*y)
        return np.r_[ode_rhs(y, k, g, Q, np.zeros_like(y)),
                     jac@z + f_theta(y)]
    solution = solve_ivp(augmented, (0.0, end), np.r_[y0, np.zeros(n)],
                         method="Radau", rtol=RTOL, atol=ATOL)
    assert solution.success
    y, z = solution.y[:n, -1], solution.y[n:, -1]
    return {"first_derivative": float(pi_prime @ y + pi @ z),
            "pi_term": float(pi_prime @ y),
            "state_term": float(pi @ z), "nfev": int(solution.nfev),
            "W0_augmented": float(pi@y)}


def normalized_softmax(values, beta):
    zz = np.asarray(values, dtype=float) * beta
    e = np.exp(zz - np.max(zz))
    return (e / e.sum()).tolist()


def mixture(scores, beta):
    k = len(scores)
    sm = normalized_softmax(scores, beta)
    p = [EPSILON/k + (1-EPSILON)*v for v in sm]
    assert abs(sum(p)-1) < 2e-14 and min(p) >= EPSILON/k-1e-14
    return p


def candidate_set(cases):
    return [(label, axis) for label in LABELS for axis in AXES
            if not (axis == "CUBIC_ALL_REL" and np.max(cases[label]["g"]) == 0)]


def load_original_openevolve(vendor_root, source_zip):
    assert hashlib.sha256(Path(source_zip).read_bytes()).hexdigest() == VENDOR_ZIP_LOCK
    root = Path(vendor_root).resolve() / "openevolve"
    assert (root / "database.py").is_file()
    # Import the genuine vendor module without initializing its LLM controller.
    # The standard package __init__ loads unused LLM orchestration modules.
    mod = types.ModuleType("openevolve")
    mod.__path__ = [str(root)]
    sys.modules["openevolve"] = mod
    from openevolve.config import DatabaseConfig
    from openevolve.database import Program, ProgramDatabase
    assert Path(sys.modules["openevolve.database"].__file__).resolve() == (root / "database.py")
    return DatabaseConfig, Program, ProgramDatabase


def canonical_candidate(candidate):
    cohort, axis = candidate
    return ("asymmetric_2" if cohort == "asymmetric_2_refined" else cohort, axis)


def compute_scores(candidates, observed):
    # Source-equivalent refined controls may be measured, but never count
    # twice in posterior evidence or receive independent physical reward.
    unique = {}
    for candidate, value in observed.items():
        key = canonical_candidate(candidate)
        if key in unique:
            assert abs(unique[key]-value) < 1e-13, ("ALIAS_INCONSISTENT",key)
        unique[key] = value
    groups = defaultdict(list)
    for candidate, value in unique.items():
        groups[candidate[1]].append(value)
    axis_score = {
        axis: (ALPHA*M0 + sum(groups[axis]))/(ALPHA + len(groups[axis]))
        for axis in AXES
    }
    candidate_score = []
    for candidate in candidates:
        axis = candidate[1]
        key = canonical_candidate(candidate)
        score = (ALPHA*M0 + unique[key])/(ALPHA+1) if key in unique else M0
        candidate_score.append(0.5*axis_score[axis]+0.5*score)
    return axis_score, candidate_score


def priors(candidates, observed, corrected):
    if not corrected:
        return [1.0/len(candidates)]*len(candidates), [1.0/len(AXES)]*len(AXES)
    axis_scores, cand_scores = compute_scores(candidates, observed)
    return mixture(cand_scores, BETA_MCTS), mixture(
        [axis_scores[a] for a in AXES], BETA_MUT)


def pick_mutation(rng, pool, axis_prob, parent):
    available_axes = {a for _, a in pool}
    ws = [axis_prob[AXES.index(a)] if a in available_axes else 0.0
          for a in AXES]
    axis = rng.choices(AXES, weights=ws, k=1)[0]
    options = [pair for pair in pool if pair[1] == axis]
    prev_cohort = parent.metadata.get("cohort") if parent else None
    same = [pair for pair in options if pair[0] == prev_cohort]
    if same and rng.random() < 0.55:
        return same[0]
    return rng.choice(options)


def pick_puct(rng, pool, candidates, current_prior, observed):
    # One root expansion per wave; visited children retain their actual Q values.
    total = len(observed)
    n = {c: int(c in observed) for c in candidates}
    q = {c: observed.get(c, 0.0) for c in candidates}
    score = {
        c: q[c] + 1.8*current_prior[candidates.index(c)] *
        math.sqrt(total + 1) / (1+n[c]) for c in pool
    }
    largest = max(score.values())
    ties = [c for c in pool if abs(score[c] - largest) <= 1e-15]
    return rng.choice(ties)


def run_arm(name, corrected, cases, candidates, measured, trace, Program,
            ProgramDatabase, DatabaseConfig, physical):
    rng = random.Random(SEED)
    db = ProgramDatabase(DatabaseConfig(num_islands=1,
                          feature_dimensions=["score"], random_seed=SEED,
                          population_size=128, archive_size=64))
    seed = Program(id=f"{name}-seed", code="ORIGINAL_SOURCE_ONLY",
                   metrics={"combined_score": 0.0, "score": 0.0},
                   metadata={"cohort": None, "axis": None})
    assert db.add(seed, iteration=0) == seed.id
    observed = {}
    waves = []
    for wave in range(16):
        prior_before, mutation_before = priors(candidates, observed, corrected)
        pool = [candidate for candidate in candidates if candidate not in observed]
        assert len(pool) >= 1
        wave_kind = "OpenEvolve_mutation" if wave % 2 == 0 else "MCTS_PUCT"
        parent = None
        if wave % 2 == 0:
            parent, inspirations = db.sample(num_inspirations=0)
            candidate = pick_mutation(rng, pool, mutation_before, parent)
        else:
            candidate = pick_puct(rng, pool, candidates, prior_before, observed)
        entry, reused = measured(candidate[0], candidate[1])
        observed[candidate] = entry["reward"]
        program = Program(
            id=f"{name}-real-{wave:02d}",
            code=json.dumps({"cohort": candidate[0], "axis": candidate[1]}),
            parent_id=parent.id if parent is not None else None,
            generation=wave+1,
            metrics={"combined_score": entry["reward"], "score": entry["reward"],
                     "actual_response_s1": entry["s1"],
                     "actual_response_s2": entry["s2"]},
            metadata={"cohort": candidate[0], "axis": candidate[1],
                      "physical_data_accepted": entry["valid"],
                      "method": wave_kind},
        )
        assert db.add(program, iteration=wave+1) == program.id
        assert db.get(program.id).metrics["score"] == entry["reward"]
        prior_after, mutation_after = priors(candidates, observed, corrected)
        rec = {
            "arm": name, "wave": wave+1, "operator": wave_kind,
            "candidate": list(candidate),
            "real_experiment_reward": entry["reward"],
            "physical_experiment_cache_hit": reused,
            "program_database_parent": parent.id if parent else None,
            "program_database_inserted_id": program.id,
            "prior_before": prior_before, "prior_after": prior_after,
            "mutation_before": mutation_before, "mutation_after": mutation_after,
            "n_programs": len(db.programs),
            "physical_consensus_max": entry["consensus_max"],
            "source_equivalence": entry["source_equivalence"],
        }
        waves.append(rec)
        trace.append(rec)
    assert len(waves) == 16 and len(db.programs) >= 1, len(db.programs)
    if corrected:
        assert any(max(abs(a-b) for a,b in zip(w["prior_before"],w["prior_after"])) > 1e-7 for w in waves)
        assert any(max(abs(a-b) for a,b in zip(w["mutation_before"],w["mutation_after"])) > 1e-7 for w in waves)
    else:
        assert all(w["prior_before"] == w["prior_after"] for w in waves)
        assert all(w["mutation_before"] == w["mutation_after"] for w in waves)
    best = max(observed, key=observed.get)
    unique = {canonical_candidate(candidate): reward for candidate,reward in observed.items()}
    return {
        "arm": name, "waves": 16,
        "ProgramDatabase_real_vendor": True, "db_members": len(db.programs),
        "observed_pairs": len(observed), "distinct_physical_training_pairs": len(unique),
        "total_training_reward": float(sum(unique.values())),
        "best_candidate": list(canonical_candidate(best)),
        "best_genotype": list(best), "best_training_reward": observed[best],
        "final_prior": priors(candidates, observed, corrected)[0],
        "final_mutation": priors(candidates, observed, corrected)[1],
        "first_wave": waves[0], "last_wave": waves[-1],
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--vendor-root", required=True)
    ap.add_argument("--source-zip", required=True)
    ap.add_argument("--out", default=str(HERE))
    args = ap.parse_args()
    verify_locks()
    cases = source_cases()
    Config, Program, ProgramDatabase = load_original_openevolve(
        args.vendor_root, args.source_zip)
    candidates = candidate_set(cases)
    assert len(candidates) == 19
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    seen = {}
    physics_rows = []
    all_trajectories = []
    def measured(cohort, axis, h=H, end=T_END, n_samples=N_SAMPLES):
        # The refined source is a resolution control of asymmetric_2, not a
        # second independent IVP. Cache on physical model identity.
        physical_cohort = canonical_candidate((cohort, axis))[0]
        key = (physical_cohort, axis, h, end, n_samples)
        if key in seen:
            return seen[key], True
        result = evaluate(cases[physical_cohort], physical_cohort, axis, h, end, n_samples)
        trajectories = result.pop("trajectories")
        seen[key] = result
        physics_rows.append(result)
        for record in trajectories:
            all_trajectories.append({"cohort": physical_cohort, "axis": axis,
                                    "h": h, "horizon": end, **record})
        return result, False

    traces = []
    adaptive = run_arm("corrected", True, cases, candidates, measured, traces,
                       Program, ProgramDatabase, Config, physics_rows)
    control = run_arm("uniform_control", False, cases, candidates, measured, traces,
                      Program, ProgramDatabase, Config, physics_rows)
    holdout = []
    for arm in (adaptive, control):
        cohort, axis = arm["best_candidate"]
        entry, cache_hit = measured(cohort, axis, h=HOLDOUT_H, end=HOLDOUT_END)
        first = sensitivity(cases[cohort], axis, end=HOLDOUT_END)
        predicted_plus = first["W0_augmented"] + HOLDOUT_H*first["first_derivative"]
        predicted_minus = first["W0_augmented"] - HOLDOUT_H*first["first_derivative"]
        holdout.append({
            "arm": arm["arm"], "candidate": [cohort, axis],
            "h": HOLDOUT_H, "horizon": HOLDOUT_END,
            "predicted_W_plus_linear": predicted_plus,
            "predicted_W_minus_linear": predicted_minus,
            "measured_W_plus": entry["Wp"], "measured_W_minus": entry["Wm"],
            "max_linear_abs_error": max(abs(predicted_plus-entry["Wp"]),
                                       abs(predicted_minus-entry["Wm"])),
            "independent_variational": first,
            "cross_solver_consensus": entry["consensus_max"],
            "cache_hit": cache_hit,
        })

    wp9_regression = sensitivity(cases["asymmetric_2"], "EXCHANGE_C01_REL", 0.1)
    source_wp9_dW = -0.0169628408664
    assert abs(wp9_regression["first_derivative"]-source_wp9_dW) < 3e-8, wp9_regression

    def write_jsonl(path, rows):
        with (out / path).open("w") as f:
            for row in rows:
                f.write(json.dumps(row, sort_keys=True, separators=(",",":"),
                                   allow_nan=False) + "\n")

    write_jsonl("TRAJECTORIES.jsonl", all_trajectories)
    write_jsonl("SEARCH_TRACE.jsonl", traces)
    write_jsonl("PHYSICAL_EXPERIMENTS.jsonl", physics_rows)
    result = {
        "schema": "vigilode.wp9b.real_source_experiment_feedback.v1",
        "parent_wp9_head": "1e4acb486fe5f497594b9cf040e9b791aca1e39b",
        "pre_simulation_prereg_commit": "3734004ebaaa5b5ce65f692791faac9a25bb828b",
        "source_blobs": SOURCE_LOCKS,
        "openevolve_source_zip_sha256": VENDOR_ZIP_LOCK,
        "measurement": {
            "source_cohorts": list(LABELS), "source_unique_physical_models": 3,
            "candidate_pairs": len(candidates),
            "unique_physical_candidate_pairs": len({canonical_candidate(c) for c in candidates}),
            "theta_training": H,
            "horizon_training": T_END, "theta_holdout": HOLDOUT_H,
            "horizon_holdout": HOLDOUT_END, "rtol": RTOL, "atol": ATOL,
            "consensus_threshold_exploratory": CONSENSUS,
            "distinct_ode_candidate_experiments": len(seen),
            "source_locked_training_experiments": sum(r["h"] == H for r in physics_rows),
            "holdout_experiments": sum(r["h"] == HOLDOUT_H for r in physics_rows),
            "actual_scipy_solver_integrations": sum(r["solver_integrations"] for r in physics_rows),
            "total_nfev": sum(r["total_nfev"] for r in physics_rows),
            "actual_physical_trajectory_rows": len(all_trajectories),
            "all_physical_consensus_pass": all(r["valid"] for r in physics_rows),
            "max_cross_solver_state_gap": max(r["consensus_max"] for r in physics_rows),
        },
        "arms": [adaptive, control], "holdout": holdout,
        "wp9_first_variation_reproduction": wp9_regression,
        "wp9_reference_first_variation": source_wp9_dW,
        "external_llm_calls": 0,
        "independent_author_review": "NOT_RUN",
        "native_be_rodas5p_matched_runner": "NOT_RUN",
        "true_rei_bianchi_igm_owner_ivp": "NOT_RUN",
        "rigorous_interval": "NOT_RUN",
        "official_vigilode_dag": "7/23", "successor_admitted": "0/12",
        "production": "HOLD", "no_merge": True,
    }
    for file in ("TRAJECTORIES.jsonl", "SEARCH_TRACE.jsonl", "PHYSICAL_EXPERIMENTS.jsonl"):
        result[file+"_sha256"] = hashlib.sha256((out/file).read_bytes()).hexdigest()
    (out/"RESULTS.json").write_text(
        json.dumps(result, sort_keys=True, indent=2, allow_nan=False)+"\n")
    verify_locks()
    print("WP9B_COMPLETED", json.dumps({
        "source_verified": True, "training": result["measurement"]["source_locked_training_experiments"],
        "holdout": result["measurement"]["holdout_experiments"],
        "integrations": result["measurement"]["actual_scipy_solver_integrations"],
        "max_consensus": result["measurement"]["max_cross_solver_state_gap"],
        "adaptive_reward": adaptive["total_training_reward"],
        "control_reward": control["total_training_reward"],
        "adaptive_best": adaptive["best_candidate"],
        "control_best": control["best_candidate"],
        "holdout": [{k: v for k,v in h.items() if k in
                     ("arm", "candidate", "max_linear_abs_error")} for h in holdout],
        "result_sha256": hashlib.sha256((out/"RESULTS.json").read_bytes()).hexdigest(),
    }, sort_keys=True))


if __name__ == "__main__":
    main()
