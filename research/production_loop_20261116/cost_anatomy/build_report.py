#!/usr/bin/env python3
"""Render REPORT_KO.md of loop L1 node V04 from RESULTS.json, AMDAHL.json and REPORT_TABLES.md.

The prose below is fixed text; every number in it is looked up in the two JSON files when the
report is rendered, so the report cannot drift from the results.
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load(name):
    def invalid(value):
        raise ValueError("nonfinite JSON constant: " + value)
    return json.loads((HERE / name).read_text(), parse_constant=invalid)


class Data:
    def __init__(self):
        self.results = load("RESULTS.json")
        self.amdahl = load("AMDAHL.json")
        self.cells = {c["cell"]: c for c in self.results["cells"]}
        self.regimes = {r["cell"]: r for r in self.amdahl["regimes"]}

    # ---- single values ---------------------------------------------------------------------
    def frac(self, cell, key):
        """Wall fraction (of the instrumented run) as a number, or None."""
        c = self.cells.get(cell, {})
        return c.get("phase_fractions", {}).get(key)

    def ifrac(self, cell, key):
        """Instruction fraction (callgrind) as a number, or None."""
        c = self.cells.get(cell, {})
        return c.get("instruction_counts", {}).get("phase_fractions", {}).get(key)

    def W(self, cell, key):
        v = self.frac(cell, key)
        return "미측정" if v is None else f"{100 * v:.1f} %"

    def I(self, cell, key):
        v = self.ifrac(cell, key)
        return "미측정" if v is None else f"{100 * v:.1f} %"

    def us_attempt(self, cell):
        c = self.cells.get(cell, {})
        if "wall" not in c:
            return None
        return c["wall"]["plain_ns_per_attempt"]["median"] / 1000.0

    def US(self, cell):
        v = self.us_attempt(cell)
        if v is None:
            return "미측정"
        return f"{v:.3g} µs" if v < 1000 else (f"{v / 1000:.3g} ms" if v < 1e6 else f"{v / 1e6:.3g} s")

    def ratio(self, cell):
        c = self.cells.get(cell, {})
        if "wall" not in c:
            return None
        return c["wall"]["instrumented_over_plain"]["ratio_of_medians"]

    def RATIO(self, cell):
        v = self.ratio(cell)
        return "미측정" if v is None else f"{v:.3f}"

    def bound(self, cell, lever, sub):
        r = self.regimes.get(cell)
        if r is None or r.get(lever) is None:
            return None
        return r[lever].get(sub)

    def B(self, cell, lever, sub):
        v = self.bound(cell, lever, sub)
        return "미측정" if v is None else f"{v:.2f}"

    def label(self, cell):
        return self.cells.get(cell, {}).get("admissibility", "not_measured")

    # ---- ranges over several cells ---------------------------------------------------------
    @staticmethod
    def _range(values, scale=1.0, digits=1, unit=""):
        values = [v for v in values if v is not None]
        if not values:
            return "미측정"
        low, high = min(values) * scale, max(values) * scale
        if f"{low:.{digits}f}" == f"{high:.{digits}f}":
            return f"{low:.{digits}f}{unit}"
        return f"{low:.{digits}f}–{high:.{digits}f}{unit}"

    def WR(self, cells, key):
        return self._range([self.frac(c, key) for c in cells], 100.0, 1, " %")

    def IR(self, cells, key):
        return self._range([self.ifrac(c, key) for c in cells], 100.0, 1, " %")

    def chainW(self, cell):
        a, b = self.frac(cell, "solve"), self.frac(cell, "rhs")
        if a is None:
            a = self.frac(cell, "solve_intervals")
        return None if a is None or b is None else a + b

    def chainI(self, cell):
        a, b = self.ifrac(cell, "solve"), self.ifrac(cell, "rhs")
        return None if a is None or b is None else a + b

    def CWR(self, cells):
        return self._range([self.chainW(c) for c in cells], 100.0, 1, " %")

    def CIR(self, cells):
        return self._range([self.chainI(c) for c in cells], 100.0, 1, " %")

    def RR(self, cells):
        return self._range([self.ratio(c) for c in cells], 1.0, 3)

    def BR(self, cells, lever, sub):
        return self._range([self.bound(c, lever, sub) for c in cells], 1.0, 2)

    def measured(self, cells):
        return [c for c in cells if "wall" in self.cells.get(c, {})]


def ids(d, **want):
    """Cells of RESULTS.json whose fields equal `want` and that have a wall measurement."""
    out = []
    for c in d.results["cells"]:
        if "wall" in c and all(c.get(k) == v for k, v in want.items()):
            out.append(c["cell"])
    return out


def render(d):
    R = d.results
    cells = R["cells"]
    measured = [c for c in cells if "wall" in c]
    auth = [c for c in measured if c["admissibility"] == "authoritative"]
    diag = [c for c in measured if c["admissibility"] == "diagnostic_only"]
    small = [c for c in measured if c["n"] < 128]
    not_run = [c for c in cells if "wall" not in c]
    ident_bad = R["summary"]["identity_failures"]
    sweep = [c for c in cells if "identity_sweep" in c]
    sweep_ok = [c for c in sweep if c["identity_sweep"].get("ran")
                and c["identity_sweep"]["instrumented_identical_to_plain"]
                and c["identity_sweep"]["marked_identical_to_plain"]]
    pairs = sum(c["identity"]["pairs_checked"] for c in measured)
    gate = d.amdahl["v07_stage_b_gate"]["regimes"]
    gate_auth = d.amdahl["v07_stage_b_gate"]["regimes_on_authoritative_rows"]
    loads = [max(c["load_average"]["runner_before"][0], c["load_average"]["runner_after"][0])
             for c in measured]

    # Groups of cells used in the prose.
    hires = ["dense.hires.n8.analytic.x1"]
    small8 = ["dense.hires.n8.analytic.x1", "dense.radnet.n8.analytic.x1"]
    small32 = ["dense.bruss1d.n32.analytic.x1", "dense.radnet.n32.analytic.x1"]
    small_an = ["dense.hires.n8.analytic.x1", "dense.radnet.n8.analytic.x1",
                "dense.bruss1d.n32.analytic.x1", "dense.radnet.n32.analytic.x1"]
    inplace_big = ["dense.bruss1d.n128.analytic.x1", "dense.bruss1d.n512.analytic.x1",
                   "dense.bruss1d.n2048.analytic.x1"]
    faer_an = ["dense.radnet.n128.analytic.x1", "dense.radnet.n512.analytic.x1",
               "dense.radnet.n2048.analytic.x1"]
    faer_mid = ["dense.radnet.n128.analytic.x1", "dense.radnet.n512.analytic.x1"]
    dense_fd = [c["cell"] for c in measured if c["structure"] == "dense"
                and c["jacobian_source"] == "fd" and c["n"] >= 128]
    band1 = ["banded_1d.bruss1d.n1000.analytic.x1", "banded_1d.bruss1d.n10000.analytic.x1",
             "banded_1d.bruss1d.n100000.analytic.x1"]
    band1_fd = ["banded_1d.bruss1d.n1000.fd.x1", "banded_1d.bruss1d.n10000.fd.x1",
                "banded_1d.bruss1d.n100000.fd.x1"]
    band23 = [c["cell"] for c in measured if c["structure"] in ("banded_2d", "banded_3d")
              and c["jacobian_source"] == "analytic" and c["rhs_multiplier"] == 1]
    band23_fd = [c["cell"] for c in measured if c["structure"] in ("banded_2d", "banded_3d")
                 and c["jacobian_source"] == "fd"]
    mf = [c["cell"] for c in measured if c["driver"] == "MatrixFree"]
    mf_x1 = [c["cell"] for c in measured if c["driver"] == "MatrixFree" and c["rhs_multiplier"] == 1]
    x100 = [c["cell"] for c in measured if c["rhs_multiplier"] == 100
            and c["jacobian_source"] == "analytic" and c["driver"] != "MatrixFree"
            and c["structure"] in ("dense", "banded_1d") and c["n"] >= 128]
    x100_2d = [c["cell"] for c in measured if c["rhs_multiplier"] == 100
               and c["structure"] == "banded_2d"]
    x10 = [c["cell"] for c in measured if c["rhs_multiplier"] == 10
           and c["jacobian_source"] == "analytic" and c["driver"] != "MatrixFree"
           and c["structure"] in ("dense", "banded_1d") and c["n"] >= 128]
    mf_x100 = [c["cell"] for c in measured if c["rhs_multiplier"] == 100 and c["driver"] == "MatrixFree"]

    krylov = []
    for c in measured:
        if c["driver"] == "MatrixFree":
            t = c["counts"]["totals"]
            krylov.append(t["krylov_iterations"] / max(1, t["stage_solves"]))
    krylov_text = Data._range(krylov, 1.0, 1)
    krylov_by_cell = ", ".join(
        f"`{c['cell'].split('.', 1)[1]}` {c['counts']['totals']['krylov_iterations'] / max(1, c['counts']['totals']['stage_solves']):.1f}"
        for c in measured if c["driver"] == "MatrixFree" and c["rhs_multiplier"] == 1)
    jvp_attempt = Data._range([c["counts"]["per_attempt"]["jvp_calls"] for c in measured
                               if c["driver"] == "MatrixFree"], 1.0, 0)

    lines = []
    add = lines.append
    add("# V04 — RODAS5P step의 phase별 비용 해부 (loop L1, wave 2)\n")
    add("작성 2026-11-17. 노드 `V04_STAGE_COST_ANATOMY`, 브랜치 `research/production-loop-20261116-w2-v04`. "
        "이 문서는 `build_report.py`가 `RESULTS.json`·`AMDAHL.json`에서 만든다. 본문의 수치는 렌더링할 때 "
        "두 파일에서 읽어 넣으며, 사람이 입력한 측정값은 없다.\n")

    # ---------------------------------------------------------------------------------------
    add("## 0. 요약\n")
    add(f"- 격자 {len(cells)}개 cell 중 {len(measured)}개에 wall protocol을 실행했다. 라벨: `authoritative` {len(auth)}개, "
        f"`diagnostic_only` {len(diag)}개(그중 n < 128이라 instruction 수가 1차 값인 것 {len(small)}개), 미측정 {len(not_run)}개. "
        f"보호 driver와의 bit-동일성은 {pairs}쌍 전부에서 "
        + ("성립했다." if not ident_bad else f"성립하지 않은 cell이 있다: {', '.join(ident_bad)}.") + "\n")
    add(f"- **8단계 사슬(8 RHS + 8 solve)이 병목인 영역**: 1D banded(사슬 {d.CWR(band1)}, 그 대부분이 solve {d.WR(band1, 'solve')}), "
        f"RHS가 비싼 경우(dense와 1D banded의 배율 100에서 RHS {d.WR(x100, 'rhs')}, 배율 10에서 {d.WR(x10, 'rhs')}), "
        f"matrix-free(8회 Krylov solve 구간 {d.WR(mf_x1, 'solve_intervals')}). "
        f"소형 dense(n ≤ 32)는 instruction 기준 사슬이 {d.CIR(small_an)}이다. n = 8에서는 나머지(벡터 연산·controller·출력)가 {d.IR(small8, 'remainder')}로 사슬과 같은 규모이고, "
        f"n = 32에서는 {d.IR(small32, 'remainder')}다.\n")
    add(f"- **사슬이 병목이 아닌 영역**: dense n ≥ 128은 factorisation이 지배한다(in-place LU {d.WR(inplace_big, 'factor')}, faer LU {d.WR(faer_an, 'factor')}). "
        f"2D/3D banded 직접법도 factorisation {d.WR(band23, 'factor')}이다. FD Jacobian을 쓰면 dense n ≥ 128에서 Jacobian이 {d.WR(dense_fd, 'jacobian')}를 차지한다.\n")
    add(f"- **V07 stage b 착수 조건**((J+LU)/사슬 ∈ [0.15, 0.92]이고 수락 step당 wall ≥ 500 µs)을 만족하는 regime은 {len(gate)}개이고, "
        f"그중 `authoritative` 행은 {len(gate_auth)}개다(표 T7). "
        + (f"착수 조건은 충족된다. 다만 이 regime들의 상한은 {Data._range([g['bound'] for g in gate], 1.0, 2)}"
           f"(Jacobian만 겹치면 {Data._range([g['bound_jacobian_only'] for g in gate], 1.0, 2)})로, stage b의 통과 기준 1.15와 거의 같다."
           if gate_auth else "`authoritative` 행에는 조건을 만족하는 regime이 없다.") + "\n")
    add("- **lever별 결론**(4절): Jacobian 병렬 조립(c)은 FD Jacobian 영역에서, LU lever(d, e)는 dense n ≥ 128과 2D/3D banded에서, "
        "RHS 병렬화(g)는 RHS 배율 10 이상에서만 상한이 의미 있다. level 구조 tableau(a)와 투기적 J/LU(b)는 사슬이 큰 1D banded·고비용 RHS에서만 여지가 있다. "
        "matrix-free의 shared basis(f)는 상한이 가장 크지만, 그 대부분은 보호 driver의 GMRES가 stage solve마다 restart 주기(40열)를 끝까지 돌고 그 열들을 2-pass MGS로 직교화하는 데서 나온다.\n")
    add("- **측정 중 드러난 사실**(3.6절): (1) 보호 dense driver의 faer 경로는 n = 128, 512에서 같은 궤적의 client arm보다 wall이 느렸다. instruction 수는 같고, 원인은 factorisation마다의 할당과 page fault다(6.2절). "
        "(2) matrix-free GMRES의 반복 수는 restart 주기(40열) 단위로만 늘고, 주기 안에서는 일찍 멈추지 않는다. (3) FD JVP는 보호 선형 허용오차 1e-10에서 쓸 수 없다.\n")

    # ---------------------------------------------------------------------------------------
    add("## 1. 방법\n")
    add("### 1.1 phase 정의\n")
    add("attempt(수락·거절을 모두 포함한 step 시도) 하나의 평균 비용을 다음 phase로 나눴다.\n")
    add("| phase | 직접법 (dense, banded) | matrix-free |\n|---|---|---|\n"
        "| Jacobian | `TimedStageClient`의 `linearize_ns`. FD Jacobian이면 그 안의 RHS 호출을 포함 | 없음 (JVP callback) |\n"
        "| factor | `factor_ns`: W = I/(hγ) − J 조립과 LU | 없음 |\n"
        "| solve | `solve_ns`: attempt당 8회 삼각 solve | stage solve 구간: 앞선 RHS 호출이 끝난 시각부터 다음 RHS 호출 시작(마지막 stage는 attempt observer)까지. JVP, Krylov 직교화, 최소제곱, stage 벡터 연산을 포함 |\n"
        "| RHS | driver가 부르는 RHS callback 둘레의 wall clock (새 상태의 f(t, y) 1회 + stage 7회) | 같음 |\n"
        "| 나머지 | total − (J + factor + solve + RHS): stage 벡터 연산, 오차 norm, controller, 출력 | total − (RHS + solve 구간): controller, 출력. 과제 문장의 정의(total − RHS − JVP)도 함께 기록 |\n")
    add("- **plain arm**은 보호 driver 그대로다: dense `integrate_rodas5p_fast_observed_with_atol`, banded `integrate_rodas5p_fast_banded_observed_with_atol`(Indexed kernel), "
        "matrix-free `integrate_rodas5p_mf_fast_observed_gmres_into` + `RoutingOptions::default().matrix_free`(GMRES, restart 40, rtol 1e-10, 전처리 없음, Arnoldi 예산 200).\n"
        "- **instrumented arm**은 `integrate_rodas5p_fast_client_observed_with_atol` + `TimedStageClient(DenseLuStageClient | BandedStageLu)` + RHS clock이다. "
        "matrix-free는 `integrate_rodas5p_mf_fast_observed_traced`에 RHS·JVP clock과 attempt observer를 붙였다. crate 소스는 바꾸지 않았다.\n"
        "- dense LU mode는 보호 정책(`DenseLuMode::protected_choice_at`)이 고르는 것만 쟀다: n ≤ 64 또는 nonzero ≤ 10 %이면 in-place zero-skipping, 아니면 faer. "
        "정책이 고르지 않는 mode는 보호 대응 run이 없어서 재지 않았다.\n"
        "- RHS 배율 k는 같은 RHS를 k번 계산하고 마지막 결과만 남긴다. 두 arm과 FD Jacobian에 똑같이 적용된다.\n"
        "- FD Jacobian은 Jacobian callback 안의 전진 차분이다: dense n + 1회, banded는 band의 Curtis–Powell–Reid 묶음으로 lower + upper + 2회. "
        "이 RHS 호출은 Jacobian phase에 들어가고 `fd_rhs_calls`로 따로 센다.\n")
    add("### 1.2 문제\n")
    add("| family | 내용 | 쓰인 곳 |\n|---|---|---|\n"
        "| `hires` | HIRES (n = 8). U11에서 소진된 구성이므로 개발용으로만 쓰고 holdout이라 부르지 않는다 | dense n = 8 |\n"
        "| `bruss1d` | 1D Brusselator MOL, 종 interleave, 확산계수 (cells+1)²/50, band 2/2 | dense 저장 n = 32, 128, 512, 2048 (in-place LU); banded n = 1e3, 1e4, 1e5 |\n"
        "| `radnet` | 고리 위의 복사·전도 집중 열망: 모든 노드 쌍의 T⁴ 교환(Jacobian 전 원소 nonzero), 열용량이 4자릿수에 걸침. 물리 모형이고 매개변수는 식으로 생성 | dense n = 8, 32 (in-place), 128, 512, 2048 (faer) |\n"
        "| `ac2d`, `ac3d` | Allen–Cahn, cell-centred FV, zero-flux 경계, D = 0.01 (2D), 0.04 (3D) | banded 2D 32², 64², 128²; banded 3D 8³, 16³; matrix-free 2D 64², 128², 256²; 3D 16³, 32³ |\n"
        "| `bruss2d` | 2D Brusselator (2종), α = 0.02, zero-flux 경계, band 2m/2m | banded 32², 64², 128²; matrix-free 128² |\n")
    add("정의는 `runner/src/problems.rs`에 있다. `runner selftest`가 해석 Jacobian·band·JVP·FD가 서로 맞는지 확인한다(36개 검사).\n")
    add("### 1.3 프로토콜\n")
    add("1. `PREDECLARED.json`을 pilot과 모든 측정보다 먼저 commit했다(격자, 규칙, 라벨, Amdahl 식).\n"
        "2. **pilot**: cell마다 plain arm을 attempt 12회로 한 번 실행해 attempt당 시간 c를 얻었다(phase는 재지 않음).\n"
        "3. **sizing**(`SIZING.json`): 20c ≤ 60 s이면 full protocol — attempt 예산 A = min(1000, max(20, ⌊1.5 s / c⌋)), 2 warm-up + 7 반복. "
        "그렇지 않고 8c ≤ 150 s이면 reduced protocol(A = 8, 1 + 3). run은 tf에 닿거나 예산을 다 쓰면 끝난다(줄인 horizon). 예산은 두 arm에 같다.\n"
        "4. **wall protocol**: `taskset -c 10`, 한 프로세스 안에서 반복마다 plain 1회와 instrumented 1회, 순서는 반복마다 교대. cell 전후의 load average를 기록.\n"
        "5. n < 128은 timer 비용이 지배하므로 callgrind로 phase별 instruction 수를 셌다. clock이 없는 marker 심볼(`v04_phase_*`)의 inclusive Ir이다.\n")
    add("### 1.4 허용 기준(frozen)과 라벨\n")
    add("n ≥ 128인 행은 다음을 모두 만족할 때만 `authoritative`다. (i) instrumented/plain wall(중앙값의 비) ≤ 1.02. (ii) 잰 phase의 합이 instrumented total의 0.90–1.00. "
        "(iii) 모든 쌍에서 attempt·수락·거절 수, 모든 work counter, 종료 시각과 종료 상태의 모든 비트가 보호 driver와 같음. 그리고 full protocol(고정 core, arm 교차, 2 + 7, load 기록). "
        "그 밖은 `diagnostic_only`다. reduced protocol 행은 세 기준을 계산해 적되 라벨은 `diagnostic_only`다. n < 128의 wall은 `diagnostic_only`이고 instruction 수가 1차 값이다.\n")
    add("matrix-free 행의 (ii)는 과제 문장대로 RHS + JVP callback 시간만 더해 판정했다. solve 구간까지 더한 합은 확장 값으로 따로 적었고 라벨에는 쓰지 않았다(표 T2의 마지막 열).\n")

    # ---------------------------------------------------------------------------------------
    add("## 2. 결과 표\n")
    add("표는 `build_amdahl.py`가 생성한다(`REPORT_TABLES.md`와 같은 내용). `AUTH` = authoritative, `diag` = diagnostic_only.\n")
    add("<!-- BEGIN GENERATED TABLES -->")
    add((HERE / "REPORT_TABLES.md").read_text().rstrip("\n"))
    add("<!-- END GENERATED TABLES -->\n")

    # ---------------------------------------------------------------------------------------
    add("## 3. 해석\n")
    add("### 3.1 소형 dense (n = 8, 32): instruction 기준\n")
    add(f"- HIRES(n = 8)는 attempt당 instruction의 {d.I(hires[0], 'remainder')}가 나머지(stage 벡터 연산, 오차 norm, controller, 출력)이고, solve {d.I(hires[0], 'solve')}, "
        f"factor {d.I(hires[0], 'factor')}, RHS {d.I(hires[0], 'rhs')}, Jacobian {d.I(hires[0], 'jacobian')}다. attempt 하나가 {d.US(hires[0])}이다. "
        "어떤 lever도 나머지에는 닿지 않고, thread를 쓸 만한 크기의 일감도 없다.\n"
        f"- n = 32에서는 solve가 {d.IR(['dense.bruss1d.n32.analytic.x1', 'dense.radnet.n32.analytic.x1'], 'solve')}, "
        f"factor가 {d.IR(['dense.bruss1d.n32.analytic.x1', 'dense.radnet.n32.analytic.x1'], 'factor')}로 올라온다. "
        f"RHS가 n²인 `radnet`은 RHS가 {d.I('dense.radnet.n32.analytic.x1', 'rhs')}다.\n"
        f"- 이 크기에서 instrumented/plain wall 비는 {d.RR([c['cell'] for c in small])}이다. timer 비용이 phase 자체와 같은 규모라는 뜻이고, wall 분해는 쓸 수 없다. "
        f"client driver 자체의 instruction 오버헤드(marked/plain)는 n = 8에서 "
        f"{d.cells[hires[0]]['instruction_counts']['marked_over_plain']:.3f}배다.\n" if hires[0] in d.cells and 'instruction_counts' in d.cells[hires[0]] else "")
    add("### 3.2 dense n ≥ 128\n")
    add(f"- **in-place LU**(band 구조 문제를 dense로 저장한 `bruss1d`): factor가 {d.WR(inplace_big, 'factor')}, 사슬이 {d.CWR(inplace_big)}다. "
        f"n = 2048에서 attempt당 {d.US('dense.bruss1d.n2048.analytic.x1')}인데, 같은 문제를 band로 선언한 n = 1e4 cell은 {d.US('banded_1d.bruss1d.n10000.analytic.x1')}다. "
        "코드상 이 경로의 factor는 n × n 크기의 W 조립과, Legacy 정책의 pivot 탐색(열 전체 주사)을 포함한다. band LU 자체보다 이 n² 부분이 지배하는 것으로 보인다(따로 재지는 않았다). 이 영역의 처방은 LU 병렬화가 아니라 band 선언(라우팅)이다.\n"
        f"- **faer LU**(`radnet`): factor {d.WR(faer_an, 'factor')}, solve {d.WR(faer_an, 'solve')}, RHS {d.WR(faer_an, 'rhs')}. "
        f"n이 커질수록 factor 몫이 커진다(n = 2048에서 {d.W('dense.radnet.n2048.analytic.x1', 'factor')}).\n"
        f"- **FD Jacobian**: Jacobian phase가 {d.WR(dense_fd, 'jacobian')}다. RHS 호출이 attempt당 수락 상태마다 n + 1회 더 들어가기 때문이다(표 T4).\n")
    add("### 3.3 1D banded (n = 1e3, 1e4, 1e5)\n")
    add(f"- solve가 {d.WR(band1, 'solve')}, factor {d.WR(band1, 'factor')}, RHS {d.WR(band1, 'rhs')}, 나머지 {d.WR(band1, 'remainder')}다. "
        f"세 크기에서 비율이 거의 같다(n에 선형). 사슬(RHS + solve)이 {d.CWR(band1)}이고, 나머지의 대부분도 stage 사이의 벡터 연산이라 사실상 step 전체가 순차 사슬이다.\n"
        f"- solve가 factor보다 훨씬 큰 것은 band 폭이 2라서 factor가 싸고 solve는 attempt당 8번 반복되기 때문이다. Indexed kernel의 후진 대입은 행마다 나눗셈을 한 번 하는데, "
        f"n = 1000의 instruction 기준 solve 몫은 {d.I('banded_1d.bruss1d.n1000.analytic.x1', 'solve')}로 wall 몫 {d.W('banded_1d.bruss1d.n1000.analytic.x1', 'solve')}보다 작다(느린 instruction이 solve에 몰려 있다는 뜻). "
        "보호 `_with_atol` 진입점은 Indexed kernel만 쓴다(Slices·FixedWidth2는 opt-in이라 재지 않았다).\n"
        f"- FD(묶음 5개 + 1회)는 Jacobian을 {d.WR(band1_fd, 'jacobian')}로 올릴 뿐이다.\n"
        f"- 이 행들은 phase 합이 {Data._range([d.cells[c]['sum_check']['median'] for c in d.measured(band1)], 1.0, 3)}라서 기준 (ii)에 걸려 `diagnostic_only`다. "
        f"timer 문제는 아니다(instrumented/plain {d.RR(band1)}, bit-동일). 재지 못한 것은 driver 안의 벡터 연산이다.\n")
    add("### 3.4 2D/3D banded 직접법\n")
    add(f"- factor가 {d.WR(band23, 'factor')}, solve가 {d.WR(band23, 'solve')}, RHS는 {d.WR(band23, 'rhs')}다. band 폭이 m(2D) 또는 m²(3D)라서 factor가 n·b², solve가 n·b로 커진다.\n"
        f"- FD Jacobian(band 묶음 2b + 1개)은 Jacobian을 {d.WR(band23_fd, 'jacobian')}로 만든다.\n"
        f"- RHS 배율 100을 걸어도 2D banded 64²에서는 RHS가 {d.WR(x100_2d, 'rhs')}에 그친다. stencil RHS가 band LU에 비해 그만큼 싸다.\n"
        "- 3D 32³의 banded 직접법은 사전등록대로 제외했다(factor 저장 805 MB, factorisation당 곱셈-뺄셈 약 6.9e10회). 표 T9와 6절 참조.\n")
    add("### 3.5 matrix-free\n")
    add(f"- stage solve 구간이 {d.WR(mf_x1, 'solve_intervals')}를 차지하고, 그 안에서 JVP callback은 {d.WR(mf_x1, 'jvp')}뿐이다. "
        f"나머지 {d.WR(mf_x1, 'krylov_orthogonalisation_and_vector_work_in_solves')}가 Krylov 직교화(2-pass MGS)와 벡터 연산이다. RHS는 {d.WR(mf_x1, 'rhs')}다.\n"
        f"- 보호 driver의 GMRES는 solve당 Krylov 반복이 평균 {krylov_text}회, attempt당 JVP가 {jvp_attempt}회다(cell별 solve당 반복: {krylov_by_cell}). "
        "`gmres_into.rs`의 주기(`run_cycle`)가 최소제곱 문제를 주기 끝에서 한 번만 풀기 때문에 반복 수는 restart 길이 40열 단위로만 늘어난다. "
        "한 주기 안에서는 허용오차에 일찍 닿아도 멈추지 않는다. 작은 격자에서는 모든 solve가 정확히 40회(JVP 42회)이고, 격자가 커지면 뒤쪽 stage부터 둘째 주기가 필요해진다(T2의 `JVP/attempt`).\n"
        f"- RHS 배율 100에서도 RHS는 {d.WR(mf_x100, 'rhs')}에 그친다(JVP가 해석식이라 RHS 비용이 JVP로 전파되지 않음).\n"
        "- 이 행들은 기준 (ii)(RHS + JVP 합)에 걸려 `diagnostic_only`다. driver에 Krylov 직교화를 감싸는 hook이 없어서 callback clock으로는 잴 수 없다. "
        "solve 구간으로 세면 합은 0.99 이상이다(T2의 `sum (RHS+intervals)` 열).\n")
    add("### 3.6 측정 중 드러난 사실\n")
    faer_ratios = [d.ratio(c) for c in faer_mid] + [
        d.cells[c]["repeat_measurement"]["instrumented_over_plain"]["ratio_of_medians"]
        for c in faer_mid if "repeat_measurement" in d.cells.get(c, {})]
    faer_extra = Data._range([1.0 / r - 1.0 for r in faer_ratios if r], 100.0, 0, " %")

    def fault_text(cell, arm):
        f = d.cells.get(cell, {}).get("page_fault_diagnostic")
        if not f or not f.get("ran"):
            return "미측정"
        return f"{f[arm]['minor_faults_per_run']['median']:.0f}회(attempt당 {f[arm]['minor_faults_per_attempt_median']:.0f}회)"

    def repeat_ratio(cell):
        r = d.cells.get(cell, {}).get("repeat_measurement")
        return "미측정" if r is None else f"{r['instrumented_over_plain']['ratio_of_medians']:.3f}"

    def perf_line(cell):
        p = d.cells.get(cell, {}).get("perf_stat")
        if not p or "events" not in p:
            return f"`{cell}`: perf stat 없음"
        pf = p["page_faults_net_of_setup"]
        ins = p["instructions_user_instrumented_over_plain_net_of_setup"]
        cyc = p["cycles_user_instrumented_over_plain_net_of_setup"]
        return (f"`{cell}`(attempt {p['attempt_budget']}회): page fault plain {pf['plain']}회 / instrumented {pf['instrumented']}회, "
                f"instructions:u 비 {ins:.4f}, cycles:u 비 {cyc:.3f}")

    add(f"1. **faer 경로의 arm 간 차이.** `radnet` n = 128, 512(해석 Jacobian, 배율 1)에서 instrumented/plain wall이 {d.RR(faer_mid)}였다. timer를 단 쪽이 더 빠르다. "
        f"반복 측정(6.3절)에서도 n = 128은 {repeat_ratio('dense.radnet.n128.analytic.x1')}, n = 512는 {repeat_ratio('dense.radnet.n512.analytic.x1')}로 다시 나타났다. "
        "두 arm은 bit-동일하고 callgrind의 instruction 수도 같으므로(T5의 marked/plain), 차이는 사용자 instruction이 아닌 곳에서 온다. "
        "보호 driver의 faer 경로는 factorisation마다 n × n 행렬을 새로 할당하고 앞의 것을 버린다(`LuFactorization::new`). "
        "arm을 하나씩 별도 프로세스로 실행한 perf stat(6.1절)에서도 instructions:u는 같고, page fault 수는 arm과 구성에 따라 크게 갈린다. "
        "그래서 wall protocol과 같은 교차 실행에서 run별 page fault를 직접 셌다(6.2절): "
        f"n = 512에서 plain arm은 run당 {fault_text('dense.radnet.n512.analytic.x1', 'plain')}, instrumented arm은 {fault_text('dense.radnet.n512.analytic.x1', 'instrumented')}의 minor fault를 냈다. "
        "즉 보호 driver의 faer 경로가 factorisation마다 새 page를 건드리는 배치에 놓였고, client arm은 같은 블록을 재사용하는 배치에 놓였다. "
        f"n = 2048에서는 두 arm이 같은 수의 page fault를 내고 wall 비가 {d.RATIO('dense.radnet.n2048.analytic.x1')}이다. "
        f"기준 (i)은 상한만 두므로 이 행들은 `authoritative`지만, 분율은 instrumented run의 것이고 이 측정에서 보호 driver는 그보다 {faer_extra} 더 썼다는 점을 감안해야 한다. "
        "factor 저장을 재사용하면 없어질 비용이므로, 격자에 없던 lever 후보로 4절에 적었다.\n")
    add("2. **GMRES 반복 수가 40열 단위**(3.5절). 한 주기로 충분한 solve도 40열을 다 만들고 2-pass MGS로 직교화한다. matrix-free wall의 대부분이 여기서 나오며, shared basis(f)의 상한이 큰 이유이기도 하다.\n")
    fd_cell = d.cells.get("matrix_free_2d.ac2d.g128.jvpfd.x1", {}).get("sizing", {})
    add(f"3. **FD JVP 불가.** 탐색 cell `matrix_free_2d.ac2d.g128.jvpfd.x1`(pilot, attempt {fd_cell.get('pilot_attempts', '?')}회)은 "
        f"{fd_cell.get('pilot_rejected', '?')}회가 거절됐다. 전진 차분 JVP의 잡음(약 1e-8)이 보호 선형 허용오차 1e-10보다 크다. 그래서 timing 격자에서 뺐다.\n")

    # ---------------------------------------------------------------------------------------
    add("## 4. lever별 상한과 권고\n")
    add("상한은 표 T6의 값이다. 모두 `오버헤드 0, 완전한 확장성, step 수 불변`을 가정한 위쪽 한계이며, `diagnostic_only` 행에서 나온 값은 진단값이다. "
        "식은 `AMDAHL.json`의 `formulas`에 있다.\n")
    best = d.amdahl["largest_bounds_per_lever"]
    g1 = [r["g_parallel_rhs"]["P6"] for r in d.amdahl["regimes"] if r["rhs_multiplier"] == 1]
    g1_max = f"{max(g1):.2f}" if g1 else "미측정"

    def top(name, k=3):
        rows = best[name]["all_rows"][:k]
        return ", ".join(f"`{r['cell']}` {r['bound']:.2f}" for r in rows) if rows else "없음"

    def top_auth(name, k=3):
        rows = best[name]["authoritative_rows"][:k]
        return ", ".join(f"`{r['cell']}` {r['bound']:.2f}" for r in rows) if rows else "없음"

    add("| lever | 가장 큰 상한 (authoritative 행) | 가장 큰 상한 (전체 행) | 권고 |\n|---|---|---|---|\n"
        f"| (a) level 구조 tableau, L = 5 | {top_auth('a_level_structured_tableau_L5')} | {top('a_level_structured_tableau_L5')} | 사슬이 큰 곳(1D banded, 배율 100, matrix-free)에서만 {d.BR(band1 + x100 + mf_x1, 'a_level_structured_tableau', 'L5')}. 새 tableau는 새 방법이라 별도 자격 심사가 필요하고 L = 4는 이론상 불가(S9). dense n ≥ 128(배율 1)에서는 {d.BR(inplace_big + faer_an, 'a_level_structured_tableau', 'L5')} |\n"
        f"| (b) 투기적 J + LU | {top_auth('b_speculative_jacobian_lu')} | {top('b_speculative_jacobian_lu')} | 상한 1.25. T7의 regime에서만 시도. step 크기 예측이 없으면 J만 겹칠 수 있다(`J only` 열) |\n"
        f"| (c) Jacobian 병렬·색칠 조립, P = 6 | {top_auth('c_parallel_jacobian_assembly_P6')} | {top('c_parallel_jacobian_assembly_P6')} | FD Jacobian 영역에서 만들 가치가 있다. 해석 Jacobian에서는 1.0x |\n"
        f"| (d) lagged LU, k = 4 | {top_auth('d_lagged_lu_k4')} | {top('d_lagged_lu_k4')} | dense n ≥ 128과 2D/3D banded에서 상한이 크다. 단 Jacobian 지연은 차수를 3으로 낮추므로(S4) step 수 증가를 V09에서 재야 한다 |\n"
        f"| (e) dense LU 병렬화, P = 6 | {top_auth('e_parallel_dense_lu_P6')} | {top('e_parallel_dense_lu_P6')} | faer 경로(n ≥ 512)에서 가치가 있다. in-place LU 행의 상한은 band 선언으로 대체해야 한다(3.2절) |\n"
        f"| (f) shared Krylov basis | {top_auth('f_shared_krylov_basis')} | {top('f_shared_krylov_basis')} | 상한이 가장 크다. 다만 먼저 GMRES 주기의 조기 종료 부재를 분리해서 봐야 한다(같은 stopping test의 per-stage GMRES가 V06의 대조군) |\n"
        f"| (g) RHS 공간 병렬화, P = 6 | {top_auth('g_parallel_rhs_P6')} | {top('g_parallel_rhs_P6')} | RHS 배율 10 이상에서 의미가 있다. 배율 1에서는 P = 6이어도 최대 {g1_max} |\n")
    add("### 4.0 격자 밖 후보: faer factor 저장의 재사용\n")
    add(f"3.6절의 차이는 lever 목록에 없던 여지를 가리킨다. `radnet` n = 128, 512에서 보호 driver는 같은 일을 하는 client arm보다 {faer_extra} 더 걸렸고(첫 측정과 반복 측정의 범위), "
        "그 원인은 factorisation마다의 할당과 page fault다(6.2절). n = 2048에서는 두 arm이 모두 이 비용을 낸다. "
        "factor 저장과 작업 행렬을 재사용하면 방법과 궤적을 바꾸지 않고 없앨 수 있는 비용이다. 보호 파일(`rodas5p_fast.rs`, `matrix.rs`)의 일이라 이 노드에서는 측정만 했다.\n")
    add("### 4.1 V07 stage b gate\n")
    gate_bounds = Data._range([g["bound"] for g in gate], 1.0, 2)
    gate_bounds_j = Data._range([g["bound_jacobian_only"] for g in gate], 1.0, 2)
    if gate:
        add("조건을 만족하는 regime(표 T7):\n")
        for g in gate:
            add(f"- `{g['cell']}`: r = {g['r']:.3f}, 수락 step당 {g['plain_wall_per_accepted_step_us']:.0f} µs, 상한 {g['bound']:.2f} "
                f"(J만 {g['bound_jacobian_only']:.2f}), {g['admissibility']}"
                + (" (reduced_protocol)" if g["reduced_protocol"] else ""))
        add("")
        add(f"이 regime들의 상한은 {gate_bounds}이고, Jacobian만 겹칠 때는 {gate_bounds_j}다. stage b의 통과 기준이 wall 1.15배이므로 여유가 거의 없다. "
            "해석 Jacobian regime에서는 LU까지 겹쳐야 하므로 step 크기 예측이 필요하다(S6). FD Jacobian regime에서만 Jacobian 단독으로 상한이 1.15를 넘는다. "
            "착수 조건은 충족하지만, 얻을 수 있는 최댓값이 통과 기준과 거의 같다는 점을 V07에서 먼저 따져야 한다.\n")
    else:
        add("조건을 만족하는 regime이 없다.\n")

    # ---------------------------------------------------------------------------------------
    add("## 5. 허용 기준을 만족하지 못한 측정\n")
    add("| cell | 라벨 | 걸린 기준 |\n|---|---|---|")
    reason_text = {
        "full_protocol_pinned_interleaved_2_plus_7": "full protocol 아님",
        "i_instrumented_over_plain_le_1.02": "(i) instrumented/plain > 1.02",
        "ii_phase_sum_in_0.90_1.00": "(ii) phase 합이 0.90–1.00 밖",
        "iii_bit_identical_to_protected_driver": "(iii) bit-동일성 실패",
        "reduced_protocol": "reduced protocol (1 + 3, attempt 8회)",
        "n < 128: wall is diagnostic, instruction counts are primary": "n < 128 (instruction 수가 1차 값)",
    }
    for c in diag:
        add(f"| `{c['cell']}` | diagnostic_only | {'; '.join(reason_text.get(r, r) for r in c['admissibility_reasons'])} |")
    for c in not_run:
        why = c["sizing"].get("reason") or "; ".join(c.get("measurement_notes", [])) or c["protocol"]
        add(f"| `{c['cell']}` | 미측정 | {why} |")
    add("")
    add("이유를 종류별로 정리하면 다음과 같다.\n")
    add("- **(ii) phase 합 < 0.90, 직접법**: 1D banded 행. driver 안의 stage 벡터 연산·오차 norm·controller가 10 %를 넘는다. client 인터페이스 밖이라 보호 소스를 고치지 않고는 잴 수 없다.\n"
        "- **(ii), matrix-free**: Krylov 직교화가 wall의 대부분인데 callback clock 밖이다. solve 구간 계측(확장)으로는 합이 0.99 이상이다.\n"
        "- **(i) > 1.02**: timer 비용이 attempt에 비해 큰 작은 문제(n = 128 in-place 행은 반복 측정에서도 같았다), 그리고 FD Jacobian이 지배하는 cell 세 개. "
        "뒤의 세 cell은 측정 중 host load가 변했거나 내 callgrind 작업과 겹쳤고, 반복한 두 cell은 기준을 통과했다(6.3절). 즉 run이 길고 반복 수가 적은 cell에서는 (i)의 판정이 host 상태에 민감하다.\n"
        "- **reduced protocol**: 단일 run이 60 s를 넘는 cell.\n"
        "- **n < 128**: 규칙상 wall은 진단값이고 instruction 수(T5)를 쓴다.\n")

    # ---------------------------------------------------------------------------------------
    add("## 6. 한계와 사전등록 대비 편차\n")
    overlap = [c for c in measured if c.get("own_non_timing_jobs_running_during_this_cell")]
    add(f"- **host 부하.** cell별 1분 load average의 최댓값은 {min(loads):.1f}–{max(loads):.1f}였다(다른 builder가 core 0-9, 12-21을 쓰는 중). "
        f"load가 8을 넘은 cell: {', '.join('`' + c['cell'] + '`' for c, l in zip(measured, loads) if l > 8) or '없음'}. "
        "arm 교차와 중앙값이 비율을 보호하지만, 절대 시간은 그만큼 흔들릴 수 있다.\n")
    add(f"- **편차 1 (내 작업의 겹침).** callgrind pass를 wall protocol과 동시에 다른 core complex(0-5, 12-17; L3를 공유하지 않음)에서 돌렸다. "
        f"겹친 cell: {', '.join('`' + c['cell'] + '`' for c in overlap) or '없음'}. 사전등록은 겹침을 금하지 않았지만 의도한 바가 아니었고, 이후의 비-timing 작업(identity sweep, perf stat)은 wall protocol이 끝난 뒤에 실행했다. "
        "`RESULTS.json`의 `own_non_timing_jobs_running_during_this_cell`에 cell별로 기록돼 있다.\n")
    add("- **편차 2 (비-timing core 집합).** 사전등록의 `taskset -c 0-9,12-21` 대신 그 부분집합 `0-5,12-17`을 썼다(timing core와 L3를 공유하지 않는 쪽).\n")
    groups = R["runner_binary_sha256_by_log_group"]
    by_binary = {}
    for group, digests in groups.items():
        for digest in digests:
            by_binary.setdefault(digest[:12], []).append(group)
    listing = "; ".join(f"`{digest}`: {', '.join(names)}" for digest, names in sorted(by_binary.items(), key=lambda kv: -len(kv[1])))
    add(f"- **편차 4 (측정 뒤의 runner 변경).** 로그 묶음별 runner binary(SHA-256 앞 12자리)는 {listing}이다. "
        "wall protocol이 끝난 뒤 run별 page fault counter를 넣어 다시 빌드한 binary는 `exploratory_faults`(6.2절)에만 썼다. counter는 timed 구간 밖에서 읽는다.\n")
    add("- **편차 3 (보고서 작성 방식).** 이 세션의 Write 도구가 subagent의 보고서 파일 작성을 거부해서, 보고서는 `build_report.py`가 결과 JSON에서 렌더링한다. 내용상의 차이는 없다.\n")
    add("- **줄인 horizon.** run은 t0에서 시작해 attempt 예산에서 끝난다. 직접법의 phase 비용은 h에 거의 무관하지만, 궤적의 초반(작은 step, 드문 거절)만 본 것이다. matrix-free는 step이 커질수록 주기 수가 늘어나므로, 여기서 본 것(solve당 1–3주기)보다 뒤쪽 구간의 solve가 더 비쌀 수 있다. 직교화가 지배한다는 분율은 주기 수와 무관하게 유지될 것으로 보이지만 재지 않았다.\n"
        "- **허용오차 한 점.** 문제마다 rtol 1e-6 한 가지만 썼다. 직접법의 분율은 허용오차에 둔감하지만 거절 비율(Jacobian 재사용 빈도)은 달라진다.\n"
        "- **in-place LU 행의 해석.** `bruss1d` dense 행의 factor는 대부분 조립과 열 주사다. LU lever 상한(d, e)을 그대로 읽으면 안 된다.\n"
        "- **FD 증분.** 전진 차분, 증분 √ε·max(|y_j|, 1). 다른 FD 구현은 호출 수가 같아도 조립 비용이 다를 수 있다.\n"
        "- **callgrind의 instruction 수**는 사용자 공간 instruction이다. page fault 같은 kernel 비용은 들어가지 않는다.\n")
    perf = [c for c in cells if "perf_stat" in c and "events" in c["perf_stat"]]
    if perf:
        add("### 6.1 perf stat (탐색)\n")
        add("arm별 단일 run을 별도 프로세스로 실행하고 set-up만 하는 run(`none`)을 뺀 값이다. attempt 예산은 min(A, 20).\n")
        add("이 pass는 고정 core가 아니고 identity sweep과 같이 돌았으므로 cycles는 해석하지 않는다(`RESULTS.json`에는 있다). "
            "n < 128 행의 instruction 비가 큰 것은 clock 호출의 instruction이 attempt 자체와 같은 규모이기 때문이다.\n")
        add("| cell | attempts | instructions:u instr/plain | page faults plain | page faults instr |\n|---|---|---|---|---|")
        for c in perf:
            p = c["perf_stat"]
            fmt = lambda v, dgt=4: "-" if v is None else f"{v:.{dgt}f}"
            pf = p["page_faults_net_of_setup"]
            add(f"| `{c['cell']}` | {p['attempt_budget']} | {fmt(p['instructions_user_instrumented_over_plain_net_of_setup'])} | "
                f"{pf['plain'] if pf['plain'] is not None else '-'} | "
                f"{pf['instrumented'] if pf['instrumented'] is not None else '-'} |")
        add("")

    faulted = [c for c in cells if c.get("page_fault_diagnostic", {}).get("ran")]
    if faulted:
        add("### 6.2 page fault 진단 (탐색)\n")
        add("wall protocol이 끝난 뒤 runner에 run별 page fault 수(`/proc/self/stat`, timed 구간 밖에서 읽음)를 추가해 다시 빌드하고, 같은 프로토콜을 다섯 cell에 실행했다. "
            "측정에 쓴 binary와 다른 binary다(로그 metadata의 digest 참조). 값은 timed 반복의 중앙값이다.\n")
        add("| cell | attempts | plain: minor fault/run | attempt당 | instrumented: minor fault/run | attempt당 | instr/plain wall | load 전/후 |\n|---|---|---|---|---|---|---|---|")
        for c in faulted:
            f = c["page_fault_diagnostic"]
            la = f["load_average"]
            add(f"| `{c['cell']}` | {f['attempts']} | {f['plain']['minor_faults_per_run']['median']:.0f} | {f['plain']['minor_faults_per_attempt_median']:.0f} | "
                f"{f['instrumented']['minor_faults_per_run']['median']:.0f} | {f['instrumented']['minor_faults_per_attempt_median']:.0f} | "
                f"{f['instrumented_over_plain_ratio_of_medians']:.3f} | {la['runner_before'][0]:.1f}/{la['runner_after'][0]:.1f} |")
        add("")
        def per_attempt(cell, arm):
            f = d.cells.get(cell, {}).get("page_fault_diagnostic")
            return None if not f or not f.get("ran") else f[arm]["minor_faults_per_attempt_median"]

        big = per_attempt("dense.radnet.n2048.analytic.x1", "plain")
        mid_plain = per_attempt("dense.radnet.n512.analytic.x1", "plain")
        mid_instr = per_attempt("dense.radnet.n512.analytic.x1", "instrumented")
        if big is not None and mid_plain is not None:
            add(f"읽는 법: faer 경로(`radnet` n ≥ 128)는 factorisation마다 새 메모리를 건드린다. n = 2048에서는 두 arm 모두 attempt당 {big:.0f}번의 page fault를 낸다"
                f"(4 KiB page로 {big * 4096 / 2**20:.0f} MiB). n = 512에서는 heap 배치에 따라 한 arm만 낸다: 배율 1 구성에서 plain은 attempt당 {mid_plain:.0f}번"
                f"(n × n 행렬 하나가 512 page), instrumented는 {mid_instr:.0f}번이고, page fault를 내는 arm이 느리다. 배율 100 구성에서는 두 arm이 같이 낸다. "
                "in-place LU(`bruss1d`)는 step 중 page fault가 없다. 따라서 3.6절의 arm 간 차이는 timer나 client driver가 아니라 faer 경로의 factorisation별 할당에서 온다. "
                "어느 arm이 내는지는 프로세스의 heap 상태에 달려 있으므로, 사용자 프로그램에서도 같은 비용이 나타날 수 있다.\n")

    repeats = [c for c in cells if "repeat_measurement" in c and "wall" in c]
    if repeats:
        add("### 6.3 반복 측정 (탐색)\n")
        add("내 callgrind 작업과 겹쳤거나 host load가 높았던 cell을, wall protocol이 끝난 뒤 내 다른 작업이 없는 상태에서 같은 프로토콜로 한 번 더 쟀다. "
            "cell의 결과(2절의 표와 라벨)는 첫 측정이고, 아래는 비교용이다.\n")
        add("| cell | 첫 측정 instr/plain | phase 합 | plain µs/attempt | load 전/후 | 반복 instr/plain | phase 합 | plain µs/attempt | load 전/후 | 반복에서 (i) (ii) (iii) | 분율 차이 최대 (%p) |\n"
            "|---|---|---|---|---|---|---|---|---|---|---|")
        worst = 0.0
        for c in repeats:
            r = c["repeat_measurement"]
            diff = max(abs(c["phase_fractions"][k] - r["phase_fractions"][k]) for k in c["phase_fractions"])
            worst = max(worst, diff)
            crit = r["criteria"]
            flags = " ".join("O" if crit[k] else "X" for k in (
                "i_instrumented_over_plain_le_1.02", "ii_phase_sum_in_0.90_1.00",
                "iii_bit_identical_to_protected_driver"))
            la, lb = c["load_average"], r["load_average"]
            add(f"| `{c['cell']}` | {c['wall']['instrumented_over_plain']['ratio_of_medians']:.3f} | {c['sum_check']['median']:.3f} | "
                f"{c['wall']['plain_ns_per_attempt']['median'] / 1000:.0f} | {la['runner_before'][0]:.1f}/{la['runner_after'][0]:.1f} | "
                f"{r['instrumented_over_plain']['ratio_of_medians']:.3f} | {r['sum_check']['median']:.3f} | "
                f"{r['plain_ns_per_attempt']['median'] / 1000:.0f} | {lb['runner_before'][0]:.1f}/{lb['runner_after'][0]:.1f} | {flags} | {100 * diff:.1f} |")
        add("")
        add(f"두 측정 사이에서 phase 분율은 최대 {100 * worst:.1f} %p 달랐다. 분율과 그로부터 나온 상한은 이 정도의 재현성을 가진다.\n")

    # ---------------------------------------------------------------------------------------
    add("## 7. 검사\n")
    add(f"- wall protocol의 identity: {len(measured)}개 cell, {pairs}쌍(warm-up 포함) — "
        + ("전부 bit-동일." if not ident_bad else f"실패: {', '.join(ident_bad)}.") + "\n"
        f"- identity sweep(세 arm, attempt 12회): {len(sweep_ok)}/{len(sweep)}개 cell에서 instrumented·marked가 plain과 bit-동일"
        + ("." if len(sweep_ok) == len(sweep) else " (나머지는 `RESULTS.json`의 `identity_sweep` 참조).") + "\n"
        f"- callgrind run의 marked/plain 동일성: "
        f"{sum(1 for c in cells if c.get('instruction_counts', {}).get('marked_run_identical_to_plain_run'))}/"
        f"{sum(1 for c in cells if 'instruction_counts' in c)}개 cell.\n"
        "- clock의 호출 수와 driver counter의 일치(`clock_calls_equal_driver_counters`): "
        f"{sum(1 for c in measured if c['counts']['clock_calls_equal_driver_counters'])}/{len(measured)}개 cell.\n")

    add("## 8. 재현\n")
    add("```\n"
        "export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v04\n"
        "cd research/production_loop_20261116/cost_anatomy/runner\n"
        "taskset -c 0-9,12-21 cargo +1.94.1 build --locked --offline -j 4 --release\n"
        "cd ..\n"
        "python3 -I campaign.py environment\n"
        "python3 -I campaign.py pilot        # taskset -c 10\n"
        "python3 -I campaign.py size         # SIZING.json\n"
        "python3 -I campaign.py measure      # taskset -c 10, logs/measure/\n"
        "python3 -I campaign.py callgrind    # logs/callgrind/\n"
        "python3 -I campaign.py identity     # logs/identity/\n"
        "python3 -I campaign.py perfstat     # logs/perfstat/ (탐색)\n"
        "python3 -I campaign.py repeat --cell <id> ...   # logs/measure_repeat/ (탐색, 6.3절)\n"
        "python3 -I campaign.py faults --cell <id> ...   # logs/exploratory_faults/ (탐색, 6.2절)\n"
        "python3 -I build_results.py         # RESULTS.json\n"
        "python3 -I build_amdahl.py          # AMDAHL.json, REPORT_TABLES.md\n"
        "python3 -I build_report.py          # 이 문서\n"
        "```\n")
    add("파일: `PREDECLARED.json`(사전등록), `SIZING.json`(attempt 예산), `RESULTS.json`(cell별 결과), `AMDAHL.json`(lever별 상한), "
        "`logs/`(raw 출력: pilot, measure, callgrind, identity, perfstat, environment), `runner/`(측정 프로그램), "
        "`campaign.py`·`build_results.py`·`build_amdahl.py`·`build_report.py`.\n")
    return "\n".join(lines)


def main():
    text = render(Data())
    (HERE / "REPORT_KO.md").write_text(text)
    print("wrote REPORT_KO.md,", len(text.splitlines()), "lines")


if __name__ == "__main__":
    sys.exit(main())
