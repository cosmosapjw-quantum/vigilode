"""E-07: mpmath (50 digits) reconstruction of the RODAS5P ROW form exactly as
tree/crates/rodas5p-core/src/coefficients.rs:134-141 does it:
  Gamma = (I/gamma - C)^-1 ; alpha = A Gamma ; beta = alpha + Gamma ; b = Gamma^T b_code ;
  btilde = Gamma[s-1,:] ; gamma_rows = rowsum(Gamma) ; l = beta - gamma I.
Stage equation actually used (sequential.rs:335-363): Y_i = y + sum_{j<i} alpha_ij K_j,
  W K_i = h f(t + c_i h, Y_i) + h J sum_{j<i} Gamma_ij K_j + h^2 gamma_rows_i f_t,  W = I - h gamma J.
Order conditions are derived from first principles by B-series on rooted trees (no reliance on a
remembered table) and cross-checked against the Hairer-Wanner IV.7 closed forms for p<=4.
"""
import json, sys, itertools, struct, math
import mpmath as mp
mp.mp.dps = 50
TREE = "/home/cosmosapjw/.local/state/vigilode/adversarial-audit/20260927T121000Z/tree"
RUN = "/home/cosmosapjw/.local/state/vigilode/adversarial-audit/20260927T121000Z"
raw = json.load(open(f"{TREE}/fixtures/rodas5p_coefficients_snapshot.json"))
mode = sys.argv[1] if len(sys.argv) > 1 else "exact"   # exact: decimal strings parsed at 50 digits; f64: strings rounded to f64 first (as parse_number does)

def num(s):
    if '/' in s:
        a, b = s.split('/')
        return (mp.mpf(a) / mp.mpf(b)) if mode == "exact" else mp.mpf(float(a) / float(b))
    return mp.mpf(s) if mode == "exact" else mp.mpf(float(s))

g = num(raw["gamma"]); s = len(raw["c"])
A = mp.matrix([[num(x) for x in r] for r in raw["A"]])
C = mp.matrix([[num(x) for x in r] for r in raw["C"]])
c = [num(x) for x in raw["c"]]; bcode = [num(x) for x in raw["b_code"]]
I = mp.eye(s)
Ginv = I / g - C
G = mp.inverse(Ginv)
alpha = A * G
beta = alpha + G
b = [sum(G[j, i] * bcode[j] for j in range(s)) for i in range(s)]          # Gamma^T b_code
btilde = [G[s - 1, i] for i in range(s)]
bhat = [b[i] - btilde[i] for i in range(s)]                                # embedded weights (y_new - error_vector)
grows = [sum(G[i, j] for j in range(s)) for i in range(s)]
out = {"mode": mode, "dps": mp.mp.dps, "stages": s, "gamma": mp.nstr(g, 20)}

# ---- structure ----
def maxabs(vals): return max([abs(v) for v in vals] + [mp.mpf(0)])
out["struct"] = {
    "cond_Ginv_1norm": float(mp.norm(Ginv, 1) * mp.norm(G, 1)),
    "alpha_strictly_lower_max_upper_abs": float(maxabs([alpha[i, j] for i in range(s) for j in range(s) if j >= i])),
    "Gamma_lower_max_upper_abs": float(maxabs([G[i, j] for i in range(s) for j in range(s) if j > i])),
    "beta_diag_minus_gamma_max": float(maxabs([beta[i, i] - g for i in range(s)])),
    "Gamma_diag_minus_gamma_max": float(maxabs([G[i, i] - g for i in range(s)])),
    "c_minus_alpha_rowsum_max": float(maxabs([c[i] - sum(alpha[i, j] for j in range(s)) for i in range(s)])),
    "stiffly_accurate_b_minus_beta_lastrow_max": float(maxabs([b[i] - beta[s - 1, i] for i in range(s)])),
    "embedded_bhat_minus_alpha_lastrow_max": float(maxabs([bhat[i] - alpha[s - 1, i] for i in range(s)])),
    "embedded_bhat_minus_beta_row_s2_max": float(maxabs([bhat[i] - beta[s - 2, i] for i in range(s)])),
    "b_code_minus_(A_last+e_s)_max": float(maxabs([bcode[i] - (A[s - 1, i] + (1 if i == s - 1 else 0)) for i in range(s)])),
    "sum_b_minus_1": float(sum(b) - 1), "sum_bhat_minus_1": float(sum(bhat) - 1),
    "gamma_rows": [mp.nstr(v, 6) for v in grows],
}

# ---- rooted trees up to order 5 (canonical: sorted tuple of children) ----
def trees(n):
    if n == 1: return [()]
    res = set()
    # partition n-1 into multiset of subtrees
    def parts(m, maxsize):
        if m == 0: yield []; return
        for k in range(min(m, maxsize), 0, -1):
            for t in trees(k):
                for rest in parts(m - k, k):
                    yield [t] + rest
    for p in parts(n - 1, n - 1):
        res.add(tuple(sorted(p)))
    return sorted(res)
def order(t): return 1 + sum(order(c) for c in t)
def density(t):
    d = mp.mpf(order(t))
    for ch in t: d *= density(ch)
    return d
def name(t): return "t" if t == () else "[" + ",".join(name(ch) for ch in t) + "]"

memo = {}
def a_coef(i, t):
    """B-series coefficient of stage K_i for tree t (k-form, W-exact-Jacobian Rosenbrock)."""
    key = (i, t)
    if key in memo: return memo[key]
    val = mp.mpf(1)
    for ch in t:
        val *= sum(alpha[i, j] * a_coef(j, ch) for j in range(i))      # alpha strictly lower
    if len(t) == 1:
        val += sum(G[i, j] * a_coef(j, t[0]) for j in range(i + 1))    # Gamma lower incl. diagonal gamma
    memo[key] = val
    return val

alltrees = [t for n in range(1, 6) for t in trees(n)]
assert len(alltrees) == 17, len(alltrees)
conds = []
for t in alltrees:
    p = order(t); rhs = 1 / density(t)
    em = sum(b[i] * a_coef(i, t) for i in range(s)) - rhs
    ee = sum(bhat[i] * a_coef(i, t) for i in range(s)) - rhs
    conds.append({"order": p, "tree": name(t), "target": mp.nstr(rhs, 20), "resid_main": float(em), "resid_embedded": float(ee)})
out["tree_conditions"] = conds
def summarize(key, pmax):
    return {p: float(max(abs(cd[key]) for cd in conds if cd["order"] == p)) for p in range(1, pmax + 1)}
out["max_resid_main_by_order"] = summarize("resid_main", 5)
out["max_resid_embedded_by_order"] = summarize("resid_embedded", 5)

# ---- Hairer-Wanner IV.7 closed forms for p<=4 (cross-check of the tree machinery) ----
al = [sum(alpha[i, j] for j in range(s)) for i in range(s)]
bp = [sum(beta[i, j] for j in range(i)) for i in range(s)]                 # beta'_i = sum_{j<i} beta_ij
def S(w, f): return sum(w[i] * f(i) for i in range(s))
def lower(Mat, i, f): return sum(Mat[i, j] * f(j) for j in range(i))
hw = {
 "p1 sum b = 1": (S(b, lambda i: 1), 1),
 "p2 sum b beta' = 1/2-g": (S(b, lambda i: bp[i]), mp.mpf(1)/2 - g),
 "p3 sum b a^2 = 1/3": (S(b, lambda i: al[i]**2), mp.mpf(1)/3),
 "p3 sum b beta beta' = 1/6-g+g^2": (S(b, lambda i: lower(beta, i, lambda j: bp[j])), mp.mpf(1)/6 - g + g**2),
 "p4 sum b a^3 = 1/4": (S(b, lambda i: al[i]**3), mp.mpf(1)/4),
 "p4 sum b a alpha beta' = 1/8-g/3": (S(b, lambda i: al[i] * lower(alpha, i, lambda j: bp[j])), mp.mpf(1)/8 - g/3),
 "p4 sum b beta a^2 = 1/12-g/3": (S(b, lambda i: lower(beta, i, lambda j: al[j]**2)), mp.mpf(1)/12 - g/3),
 "p4 sum b beta beta beta' = 1/24-g/2+3g^2/2-g^3": (S(b, lambda i: lower(beta, i, lambda j: lower(beta, j, lambda k: bp[k]))), mp.mpf(1)/24 - g/2 + 3*g**2/2 - g**3),
}
out["hw_closed_form_main"] = {k: float(v - t) for k, (v, t) in hw.items()}
out["hw_closed_form_max_abs"] = max(abs(x) for x in out["hw_closed_form_main"].values())

# ---- f64 ulp comparison with the Rust crate output (e07_coeff_print) ----
try:
    f64 = json.load(open(f"{RUN}/exp/E-07/coeff_f64.json"))
    def bits(hexs): return struct.unpack('<d', bytes.fromhex(hexs)[::-1])[0]
    SMALL = 1e-8   # entries below this (mathematically zero / cancellation residue) are compared by absolute difference
    def ulps(x_mp, x_f64):
        if abs(x_f64) < SMALL and abs(x_mp) < SMALL: return None
        u = math.ulp(abs(x_f64)) if x_f64 != 0 else math.ulp(abs(float(x_mp)))
        return float((x_mp - mp.mpf(x_f64)) / u)
    cmp = {}
    def pack(key, pairs, shape):
        u = [ulps(a, b) for a, b in pairs]
        absd = [abs(float(a - mp.mpf(b))) for a, b in pairs]
        sig = [v for v in u if v is not None]
        small = [d for d, v in zip(absd, u) if v is None]
        cmp[key] = {"max_abs_ulp_significant": max(abs(v) for v in sig) if sig else 0.0,
                    "n_significant": len(sig), "n_small": len(small),
                    "max_abs_diff_small_entries": max(small) if small else 0.0,
                    "max_abs_diff_all": max(absd), "ulps": [None if v is None else round(v, 2) for v in u]}
    def cmpmat(key, M):
        pack(key, [(M[i, j], bits(f64[key][i][j]["bits"])) for i in range(s) for j in range(s)], (s, s))
    def cmpvec(key, V):
        pack(key, [(V[i], bits(f64[key][i]["bits"])) for i in range(s)], (s,))
    cmpmat("gamma_matrix", G); cmpmat("alpha", alpha); cmpmat("beta", beta); cmpmat("l", beta - g * I)
    cmpvec("b", b); cmpvec("btilde", btilde); cmpvec("gamma_rows", grows); cmpvec("c", c); cmpvec("b_code", bcode)
    cmp["gamma"] = {"ulp": ulps(g, bits(f64["gamma"]["bits"]))}
    # order-condition residuals evaluated on the crate's actual f64 coefficients (what the code integrates with)
    Gf = mp.matrix([[mp.mpf(bits(f64["gamma_matrix"][i][j]["bits"])) for j in range(s)] for i in range(s)])
    alf = mp.matrix([[mp.mpf(bits(f64["alpha"][i][j]["bits"])) for j in range(s)] for i in range(s)])
    bf = [mp.mpf(bits(f64["b"][i]["bits"])) for i in range(s)]
    btf = [mp.mpf(bits(f64["btilde"][i]["bits"])) for i in range(s)]
    memo.clear(); alpha_save, G_save = alpha, G; alpha, G = alf, Gf
    f64conds = []
    for t in alltrees:
        rhs = 1 / density(t)
        f64conds.append({"order": order(t), "tree": name(t), "resid_main": float(sum(bf[i]*a_coef(i, t) for i in range(s)) - rhs),
                         "resid_embedded": float(sum((bf[i]-btf[i])*a_coef(i, t) for i in range(s)) - rhs)})
    alpha, G = alpha_save, G_save; memo.clear()
    cmp["f64_coefficients_max_resid_main_by_order"] = {p: max(abs(cd["resid_main"]) for cd in f64conds if cd["order"] == p) for p in range(1, 6)}
    cmp["f64_coefficients_max_resid_embedded_by_order"] = {p: max(abs(cd["resid_embedded"]) for cd in f64conds if cd["order"] == p) for p in range(1, 6)}
    out["f64_vs_mpmath"] = cmp
except FileNotFoundError:
    out["f64_vs_mpmath"] = "coeff_f64.json missing"
json.dump(out, open(f"{RUN}/exp/E-07/mpmath_{mode}.json", "w"), indent=1)
print(json.dumps({k: out[k] for k in ("mode", "struct", "max_resid_main_by_order", "max_resid_embedded_by_order", "hw_closed_form_max_abs")}, indent=1))
for cd in conds: print(f"p={cd['order']} {cd['tree']:22s} main={cd['resid_main']:+.2e} emb={cd['resid_embedded']:+.2e}")
if isinstance(out["f64_vs_mpmath"], dict):
    for k, v in out["f64_vs_mpmath"].items():
        print(k, v if not isinstance(v, dict) or "ulps" not in v else f"max_abs_ulp_significant={v['max_abs_ulp_significant']} (n={v['n_significant']}) small_entries n={v['n_small']} max_abs_diff={v['max_abs_diff_small_entries']:.2e}")
