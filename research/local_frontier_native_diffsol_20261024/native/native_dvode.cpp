// Native, single-process DVODE driver for the four declared local-frontier IVPs.
// Netlib vode.f is acquired and hashed outside the source checkout.
#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstddef>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
constexpr int max_n = 6;
constexpr double probe_limit = 128.0 * std::numeric_limits<double>::epsilon();

void rhs(int problem, double t, const double* y, double* f, const double* p) {
    switch (problem) {
    case 1: {
        const double a = p[0] * y[0], b = p[1] * y[1] * y[2], c = p[2] * y[1] * y[1];
        f[0] = -a + b; f[1] = a - b - c; f[2] = c; return;
    }
    case 10: {
        const double g = std::sqrt(1.0 + y[3]*y[3] + y[4]*y[4] + y[5]*y[5]);
        f[0] = y[3]/g; f[1] = y[4]/g; f[2] = y[5]/g;
        f[3] = p[0]*y[4]/g; f[4] = -p[0]*y[3]/g; f[5] = 0.0; return;
    }
    case 15: {
        const double a = 2.0*p[2] - p[1];
        const double b = p[0]*p[0]*std::exp(2.0*p[1]*t) + p[2]*p[2] - p[2]*p[1];
        f[0] = y[1]; f[1] = -a*y[1] - b*y[0]; return;
    }
    case 16: {
        const double q = y[0]*y[0] + y[1]*y[1];
        if (!(q > 0.0) || !std::isfinite(q)) { f[0] = f[1] = std::numeric_limits<double>::quiet_NaN(); return; }
        const double radial = -p[0]/(q*q), w = p[1]/std::pow(q, 0.75);
        f[0] = radial*y[0] - w*y[1]; f[1] = radial*y[1] + w*y[0]; return;
    }
    default: throw std::runtime_error("unsupported case");
    }
}

extern "C" void native_rhs(int*, double* t, double* y, double* out, double* p, int* ipar) {
    ++ipar[1];
    rhs(ipar[0], *t, y, out, p);
}
extern "C" void unused_jac(int*, double*, double*, int*, int*, double*, int*, double*, int*) {}
using rhs_fn = decltype(&native_rhs);
using jac_fn = decltype(&unused_jac);
extern "C" void dvode_(rhs_fn, int*, double*, double*, double*, int*, double*, double*,
                        int*, int*, int*, double*, int*, int*, int*, jac_fn, int*, double*, int*);

struct Input {
    int problem = 0, n = 0;
    std::string size;
    double t0 = 0.0, t_end = 0.0, rtol = 0.0, atol = 0.0, error_limit = 0.0;
    std::array<double, 4> p{};
    std::array<double, max_n> y{};
    int probes = 0;
    double max_probe_difference = 0.0;
};

Input read_input(const std::string& filename) {
    std::ifstream in(filename);
    if (!in) throw std::runtime_error("cannot open numeric contract");
    Input c;
    in >> c.problem >> c.n >> c.size;
    in >> c.t0 >> c.t_end >> c.rtol >> c.atol >> c.error_limit;
    for (double& v : c.p) in >> v;
    const int expected_n = c.problem == 1 ? 3 : c.problem == 10 ? 6 : (c.problem == 15 || c.problem == 16) ? 2 : 0;
    if (c.n != expected_n || c.n < 1 || c.n > max_n || !(c.t0 == 0.0 && c.t_end > c.t0 && c.rtol > 0.0 && c.atol > 0.0 && c.error_limit > 0.0))
        throw std::runtime_error("unsupported or invalid contract header");
    if (c.size != "smoke" && c.size != "standard") throw std::runtime_error("invalid size");
    for (double& v : c.p) if (!std::isfinite(v)) throw std::runtime_error("nonfinite parameter");
    for (int i = 0; i < c.n; ++i) in >> c.y[i];
    const std::array<double,max_n> initial = c.problem == 1 ? std::array<double,max_n>{1,0,0,0,0,0}
        : c.problem == 10 ? std::array<double,max_n>{0,0,0,1,0,.2}
        : c.problem == 15 ? std::array<double,max_n>{1,-.8,0,0,0,0}
        : std::array<double,max_n>{1,0,0,0,0,0};
    for (int i = 0; i < c.n; ++i) if (c.y[i] != initial[i]) throw std::runtime_error("initial state mismatch");
    if (c.problem == 16 && c.p[2] != 1.0) throw std::runtime_error("inspiral r0 mismatch");
    in >> c.probes;
    if (c.probes < 2 || c.probes > 32) throw std::runtime_error("need 2..32 RHS probes");
    for (int j = 0; j < c.probes; ++j) {
        double t = 0.0;
        std::array<double,max_n> state{}, expected{}, actual{};
        in >> t;
        for (int i = 0; i < c.n; ++i) in >> state[i];
        for (int i = 0; i < c.n; ++i) in >> expected[i];
        if (!in || !std::isfinite(t)) throw std::runtime_error("invalid RHS probe");
        rhs(c.problem, t, state.data(), actual.data(), c.p.data());
        for (int i = 0; i < c.n; ++i) {
            const double scale = std::max({1.0, std::abs(actual[i]), std::abs(expected[i])});
            const double d = std::abs(actual[i] - expected[i])/scale;
            if (!std::isfinite(d) || d > probe_limit) throw std::runtime_error("native RHS probe mismatch");
            c.max_probe_difference = std::max(c.max_probe_difference, d);
        }
    }
    if (!in) throw std::runtime_error("truncated numeric contract");
    return c;
}
} // namespace

int main(int argc, char** argv) {
    try {
        if (argc != 3 && argc != 4) throw std::runtime_error("usage: native_dvode CONTRACT.txt adams|bdf [max_steps]");
        const Input c = read_input(argv[1]);
        const std::string method = argv[2];
        if (method != "adams" && method != "bdf") throw std::runtime_error("unknown method");
        const int max_steps = argc == 4 ? std::stoi(argv[3]) : 200000;
        if (max_steps < 1 || max_steps > 2000000) throw std::runtime_error("invalid max_steps");
        int neq = c.n, itol = 1, itask = 1, istate = 1, iopt = 1, mf = method == "adams" ? 10 : 22;
        int lrw = mf == 10 ? 20 + 16*neq : 22 + 9*neq + 2*neq*neq;
        int liw = mf == 10 ? 30 : 30 + neq;
        double t = c.t0, tout = c.t_end, rtol = c.rtol, atol = c.atol;
        std::array<double,max_n> y = c.y;
        std::array<double,4> rpar = c.p;
        std::array<int,4> ipar{c.problem,0,0,0};
        const auto started = std::chrono::steady_clock::now();
        std::vector<double> rwork(static_cast<std::size_t>(lrw), 0.0);
        std::vector<int> iwork(static_cast<std::size_t>(liw), 0);
        iwork[5] = max_steps; // IWORK(6): maximum internal steps
        dvode_(native_rhs,&neq,y.data(),&t,&tout,&itol,&rtol,&atol,&itask,&istate,&iopt,
               rwork.data(),&lrw,iwork.data(),&liw,unused_jac,&mf,rpar.data(),ipar.data());
        const double elapsed = std::chrono::duration<double>(std::chrono::steady_clock::now()-started).count();
        bool finite = std::isfinite(t);
        for (int i = 0; i < neq; ++i) finite &= std::isfinite(y[i]);
        const bool ok = istate == 2 && finite && std::abs(t-tout) <= 16*std::numeric_limits<double>::epsilon()*std::max(1.0,std::abs(tout));
        std::cout << std::setprecision(17) << "{\"kind\":\"result\",\"schema_version\":1,\"case\":" << c.problem
                  << ",\"method\":\"native-dvode-" << method << "\",\"parallel_layer\":\"serial\",\"threads\":1"
                  << ",\"size\":\"" << c.size << "\",\"elapsed_seconds\":" << elapsed
                  << ",\"solver_success\":" << (ok ? "true":"false") << ",\"istate\":" << istate
                  << ",\"max_steps\":" << max_steps
                  << ",\"final_time\":" << (std::isfinite(t) ? t : c.t0) << ",\"final_state\":";
        if (finite) {
            std::cout << '[';
            for (int i = 0; i < neq; ++i) { if (i) std::cout << ','; std::cout << y[i]; }
            std::cout << ']';
        } else std::cout << "null";
        std::cout << ",\"model_match\":{\"status\":\"MATCHED_AT_DECLARED_PROBES\",\"max_scaled_probe_difference\":" << c.max_probe_difference << "}"
                  << ",\"counters\":{\"rhs_callbacks\":" << ipar[1] << ",\"steps\":" << iwork[10]
                  << ",\"rhs_evaluations\":" << iwork[11] << ",\"jacobian_evaluations\":" << iwork[12]
                  << ",\"lu_factorizations\":" << iwork[18] << ",\"nonlinear_iterations\":" << iwork[19]
                  << ",\"convergence_failures\":" << iwork[20] << ",\"error_test_failures\":" << iwork[21] << "}"
                  << ",\"timing_scope\":\"fresh native work arrays and DVODE call; input parse/probe validation/JSON excluded\"}\n";
        return ok ? 0 : 2;
    } catch (const std::exception& e) {
        std::cerr << "native_dvode: " << e.what() << '\n';
        return 3;
    }
}
