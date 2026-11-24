"""Independent oriented-face assembly and entire semidiscrete endpoint reference."""
from pathlib import Path
import json
import platform
import numpy as np
import scipy
from scipy.integrate import solve_ivp
from scipy.sparse import coo_matrix, diags
from scipy.sparse.linalg import expm_multiply

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "crates/vigilode/tests/fixtures/spatial_expm.json"


def operator(shape, lengths, kappa, radius=None):
    """Add each undirected face once. -W^-1 B^T C B fixes both orientational signs."""
    size = int(np.prod(shape))
    weight = np.zeros(size)
    faces, conducts = [], []
    for node in range(size):
        if radius is None:
            xyz = np.unravel_index(node, shape, order="F")
            h = np.array(lengths) / shape
            weight[node] = np.prod(h)
            for axis, rate in enumerate(kappa):
                other = list(xyz)
                other[axis] = (other[axis] + 1) % shape[axis]
                faces.append((node, np.ravel_multi_index(other, shape, order="F")))
                conducts.append(weight[node] * rate / h[axis]**2)
        else:
            nt, nlon = shape
            i, j = divmod(node, nlon)
            dt, dp = np.pi / nt, 2*np.pi/nlon
            weight[node] = radius**2 * dp * (np.cos(i*dt) - np.cos((i+1)*dt))
            faces.append((node, i*nlon + (j+1) % nlon))
            conducts.append(kappa*dt/(np.sin((i+0.5)*dt)*dp))
            if i+1 < nt:
                faces.append((node, node+nlon))
                conducts.append(kappa*np.sin((i+1)*dt)*dp/dt)
    nface = len(faces)
    b = coo_matrix((np.tile([-1.,1.], nface),
                    (np.repeat(np.arange(nface), 2), np.asarray(faces).ravel())),
                   shape=(nface,size)).tocsr()
    return (-diags(1/weight) @ b.T @ diags(conducts) @ b).tocsc()


def main():
    cases = [
        ("box2", ([6,5], [2.,1.], [0.2,0.08])),
        ("box3", ([4,4,5], [1.,2.,3.], [0.1,0.2,0.3])),
        ("sphere", ([6,12], None, 0.15, 1.3)),
    ]
    output = dict(reference="scipy.sparse.linalg.expm_multiply on -W^-1 B.T C B",
                  python=platform.python_version(), scipy=scipy.__version__, numpy=np.__version__,
                  final_time=0.3, rtol=1e-7, atol=1e-10, cases={})
    for name, args in cases:
        a = operator(*args)
        i = np.arange(a.shape[0], dtype=np.float64)
        y0 = np.sin(0.23*i) + 0.2*np.cos(1.93*i)
        tf = output["final_time"]
        exact = expm_multiply(tf*a, y0)
        comparators = {}
        for method in ("BDF", "Radau"):
            s = solve_ivp(lambda t, y: a@y, [0.,tf], y0, method=method, jac=a,
                          atol=output["atol"], rtol=output["rtol"])
            assert s.success and s.t[-1] == tf
            comparators[method] = dict(error_inf=float(np.max(abs(s.y[:,-1]-exact))),
                                       nfev=s.nfev, nlu=s.nlu, accepted_steps=len(s.t)-1)
        output["cases"][name] = dict(endpoint=exact.tolist(), comparators=comparators)
    OUT.write_text(json.dumps(output, indent=2)+"\n")


if __name__ == "__main__":
    main()
