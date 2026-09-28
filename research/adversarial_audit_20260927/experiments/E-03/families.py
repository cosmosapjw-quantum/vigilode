# VERBATIM COPY (by line range) of the six calibration-family definitions from the audit tree:
#   tree/tools/reference_v2/generate_references_v2.py (diversity_multiplier .. forcing_runtime) and
#   tree/tools/reference_v2/generate_references.py (semilinear_* helpers). The tree is NOT modified.
import math
import numpy as np
from scipy.sparse import csc_matrix
from typing import Any

def require(c, m):
    if not c: raise RuntimeError(m)

def diversity_multiplier(index: int) -> float:
    value = index + 1
    fraction = 0.5
    radical_inverse = 0.0
    while value:
        if value & 1:
            radical_inverse += fraction
        value >>= 1
        fraction *= 0.5
    return 0.9 + 0.2 * radical_inverse


def smooth_ramp(time_value: float, center: float, width: float) -> tuple[float, float]:
    value = math.tanh((time_value - center) / width)
    return 0.5 * (1.0 + value), 0.5 * (1.0 - value * value) / width


def sparse_from_columns(dimension: int, columns: list[tuple[int, int, float]]) -> csc_matrix:
    rows = [row for row, _, _ in columns]
    cols = [column for _, column, _ in columns]
    values = [value for _, _, value in columns]
    return csc_matrix((values, (rows, cols)), shape=(dimension, dimension))


def robertson_runtime(dimension: int):
    blocks = dimension // 3

    def rhs(time_value: float, state: np.ndarray) -> np.ndarray:
        ramp, _ = smooth_ramp(time_value, 0.045, 0.010)
        activity = 0.05 + 0.95 * ramp
        out = np.empty(dimension)
        for block in range(blocks):
            i = 3 * block
            scale = diversity_multiplier(block)
            k1, k2, k3 = 0.04 * scale, 1.0e4 * activity * scale, 3.0e7 * activity * scale
            y1, y2, y3 = state[i : i + 3]
            out[i] = -k1 * y1 + k2 * y2 * y3
            out[i + 1] = k1 * y1 - k2 * y2 * y3 - k3 * y2 * y2
            out[i + 2] = k3 * y2 * y2
        if 3 * blocks < dimension:
            out[3 * blocks :] = -20.0 * activity * state[3 * blocks :]
        return out

    def jac(time_value: float, state: np.ndarray) -> csc_matrix:
        ramp, _ = smooth_ramp(time_value, 0.045, 0.010)
        activity = 0.05 + 0.95 * ramp
        values: list[tuple[int, int, float]] = []
        for block in range(blocks):
            i = 3 * block
            scale = diversity_multiplier(block)
            k1, k2, k3 = 0.04 * scale, 1.0e4 * activity * scale, 3.0e7 * activity * scale
            y2, y3 = state[i + 1], state[i + 2]
            values += [
                (i, i, -k1), (i, i + 1, k2 * y3), (i, i + 2, k2 * y2),
                (i + 1, i, k1), (i + 1, i + 1, -k2 * y3 - 2.0 * k3 * y2),
                (i + 1, i + 2, -k2 * y2), (i + 2, i + 1, 2.0 * k3 * y2),
            ]
        return sparse_from_columns(dimension, values)

    initial = np.zeros(dimension)
    initial[0 : 3 * blocks : 3] = 1.0
    return rhs, jac, initial, 2


def hires_runtime(dimension: int):
    blocks = dimension // 8

    def rhs(time_value: float, state: np.ndarray) -> np.ndarray:
        ramp, _ = smooth_ramp(time_value, 0.45, 0.08)
        activity = 0.1 + 0.9 * ramp
        out = np.empty(dimension)
        for block in range(blocks):
            i = 8 * block
            scale = diversity_multiplier(block)
            y1, y2, y3, y4, y5, y6, y7, y8 = state[i : i + 8]
            q = 280.0 * activity * y6 * y8
            out[i] = scale * (-1.71 * y1 + 0.43 * y2 + 8.32 * y3 + 0.0007)
            out[i + 1] = scale * (1.71 * y1 - 8.75 * y2)
            out[i + 2] = scale * (-10.03 * y3 + 0.43 * y4 + 0.035 * y5)
            out[i + 3] = scale * (8.32 * y2 + 1.71 * y3 - 1.12 * y4)
            out[i + 4] = scale * (-1.745 * y5 + 0.43 * y6 + 0.43 * y7)
            out[i + 5] = scale * (-q + 0.69 * y4 + 1.71 * y5 - 0.43 * y6 + 0.69 * y7)
            out[i + 6] = scale * (q - 1.81 * y7)
            out[i + 7] = scale * (-q + 1.81 * y7)
        return out

    def jac(time_value: float, state: np.ndarray) -> csc_matrix:
        ramp, _ = smooth_ramp(time_value, 0.45, 0.08)
        activity = 0.1 + 0.9 * ramp
        values: list[tuple[int, int, float]] = []
        for block in range(blocks):
            i = 8 * block
            scale = diversity_multiplier(block)
            y6, y8 = state[i + 5], state[i + 7]
            q6 = 280.0 * activity * y8
            q8 = 280.0 * activity * y6
            rows = (
                (-1.71, 0.43, 8.32, 0, 0, 0, 0, 0),
                (1.71, -8.75, 0, 0, 0, 0, 0, 0),
                (0, 0, -10.03, 0.43, 0.035, 0, 0, 0),
                (0, 8.32, 1.71, -1.12, 0, 0, 0, 0),
                (0, 0, 0, 0, -1.745, 0.43, 0.43, 0),
                (0, 0, 0, 0.69, 1.71, -0.43 - q6, 0.69, -q8),
                (0, 0, 0, 0, 0, q6, -1.81, q8),
                (0, 0, 0, 0, 0, -q6, 1.81, -q8),
            )
            for row, entries in enumerate(rows):
                for column, value in enumerate(entries):
                    if value != 0.0:
                        values.append((i + row, i + column, scale * value))
        return sparse_from_columns(dimension, values)

    initial = np.zeros(dimension)
    initial[0 : 8 * blocks : 8] = 1.0
    initial[7 : 8 * blocks : 8] = 0.0057
    return rhs, jac, initial, 7


def vdp_runtime(dimension: int):
    blocks = dimension // 2

    def rhs(time_value: float, state: np.ndarray) -> np.ndarray:
        ramp, _ = smooth_ramp(time_value, 0.5, 0.08)
        mu = 10.0 + 490.0 * ramp
        out = np.empty(dimension)
        for block in range(blocks):
            i = 2 * block
            local_mu = mu * diversity_multiplier(block)
            out[i] = state[i + 1]
            out[i + 1] = local_mu * (1.0 - state[i] * state[i]) * state[i + 1] - state[i]
        return out

    def jac(time_value: float, state: np.ndarray) -> csc_matrix:
        ramp, _ = smooth_ramp(time_value, 0.5, 0.08)
        mu = 10.0 + 490.0 * ramp
        values = []
        for block in range(blocks):
            i = 2 * block
            local_mu = mu * diversity_multiplier(block)
            values += [
                (i, i + 1, 1.0),
                (i + 1, i, -2.0 * local_mu * state[i] * state[i + 1] - 1.0),
                (i + 1, i + 1, local_mu * (1.0 - state[i] * state[i])),
            ]
        return sparse_from_columns(dimension, values)

    initial = np.zeros(dimension)
    initial[0 : 2 * blocks : 2] = 2.0
    return rhs, jac, initial, 1


def rotating_shape(dimension: int, time_value: float, derivative: int) -> np.ndarray:
    out = np.empty(dimension)
    for index in range(dimension):
        frequency = (1.0 + float(index % 7)) * diversity_multiplier(index // 2)
        if derivative == 0:
            out[index] = 0.4 * math.sin(frequency * time_value) + 0.2 * math.cos(0.5 * frequency * time_value)
        elif derivative == 1:
            out[index] = 0.4 * frequency * math.cos(frequency * time_value) - 0.1 * frequency * math.sin(0.5 * frequency * time_value)
        else:
            out[index] = -0.4 * frequency * frequency * math.sin(frequency * time_value) - 0.05 * frequency * frequency * math.cos(0.5 * frequency * time_value)
    return out


def rotating_operator(time_value: float, values: np.ndarray) -> np.ndarray:
    ramp, _ = smooth_ramp(time_value, 0.5, 0.08)
    stiffness0 = 20.0 + 480.0 * ramp
    eta = 0.1 + 0.8 * ramp
    theta0 = 8.0 * time_value + 0.4 * math.sin(4.0 * time_value)
    out = np.empty_like(values)
    for block in range(len(values) // 2):
        i = 2 * block
        scale = diversity_multiplier(block)
        stiffness, theta = stiffness0 * scale, theta0 * scale
        cosine, sine = math.cos(theta), math.sin(theta)
        xr0 = cosine * values[i] + sine * values[i + 1]
        xr1 = -sine * values[i] + cosine * values[i + 1]
        ar0 = -stiffness * xr0 + eta * stiffness * xr1
        ar1 = -0.35 * stiffness * xr1
        out[i] = cosine * ar0 - sine * ar1
        out[i + 1] = sine * ar0 + cosine * ar1
    return out


def rotating_runtime(dimension: int):
    def rhs(time_value: float, state: np.ndarray) -> np.ndarray:
        phi = rotating_shape(dimension, time_value, 0)
        dphi = rotating_shape(dimension, time_value, 1)
        ramp, _ = smooth_ramp(time_value, 0.6, 0.06)
        return rotating_operator(time_value, state - phi) + dphi + 40.0 * ramp * (state * state - phi * phi)

    def jac(time_value: float, state: np.ndarray) -> csc_matrix:
        ramp, _ = smooth_ramp(time_value, 0.6, 0.06)
        nonlinear = 40.0 * ramp
        values: list[tuple[int, int, float]] = []
        for block in range(dimension // 2):
            i = 2 * block
            for column in range(2):
                basis = np.zeros(dimension)
                basis[i + column] = 1.0
                image = rotating_operator(time_value, basis)
                for row in range(2):
                    value = float(image[i + row])
                    if row == column:
                        value += 2.0 * nonlinear * state[i + row]
                    values.append((i + row, i + column, value))
        return sparse_from_columns(dimension, values)

    initial = rotating_shape(dimension, 0.0, 0)
    return rhs, jac, initial, 1


def forcing_runtime(dimension: int):
    def rhs(time_value: float, state: np.ndarray) -> np.ndarray:
        ramp, _ = smooth_ramp(time_value, 0.45, 0.07)
        stiffness = 30.0 + 470.0 * ramp
        frequency = 2.0 + 28.0 * ramp
        out = np.empty(dimension)
        for index in range(dimension):
            scale = diversity_multiplier(index)
            argument = frequency * time_value + float(index % 11) * 0.17
            phi = math.sin(argument)
            defect = state[index] - phi
            out[index] = -scale * stiffness * defect + scale * frequency * math.cos(argument) + 20.0 * ramp * defect * defect
        return out

    def jac(time_value: float, state: np.ndarray) -> csc_matrix:
        ramp, _ = smooth_ramp(time_value, 0.45, 0.07)
        stiffness = 30.0 + 470.0 * ramp
        frequency = 2.0 + 28.0 * ramp
        diagonal = np.empty(dimension)
        for index in range(dimension):
            scale = diversity_multiplier(index)
            phi = math.sin(frequency * time_value + float(index % 11) * 0.17)
            diagonal[index] = -scale * stiffness + 40.0 * ramp * (state[index] - phi)
        return csc_matrix((diagonal, (np.arange(dimension), np.arange(dimension))), shape=(dimension, dimension))

    initial = np.asarray([math.sin(float(index % 11) * 0.17) for index in range(dimension)])
    return rhs, jac, initial, 0

def semilinear_grid_shape(dimension: int) -> tuple[int, int]:
    grids = {96: (8, 12), 384: (16, 24), 1536: (32, 48)}
    require(dimension in grids, f"no scientific-corpus-v2.1 grid for dimension {dimension}")
    return grids[dimension]


def semilinear_exact_state(nx: int, ny: int, time: float) -> np.ndarray:
    hx = 1.0 / float(nx + 1)
    hy = 1.0 / float(ny + 1)
    decay = math.exp(-time)
    state = np.empty(nx * ny, dtype=np.float64)
    for j in range(ny):
        # Match the Rust callback's operation association exactly.  This bit
        # identity matters because the tight L2 trajectory is also the sole
        # WRMS scale anchor in the v2 reference wire format.
        y = float(j + 1) * hy
        sy = math.sin(math.pi * y)
        for i in range(nx):
            x = float(i + 1) * hx
            state[i + nx * j] = decay * math.sin(math.pi * x) * sy
    return state


def semilinear_ramp(time: float) -> tuple[float, float]:
    value = math.tanh((time - 0.5) / 0.08)
    return 0.5 * (1.0 + value), 0.5 * (1.0 - value * value) / 0.08


def semilinear_operator(nx: int, ny: int, time: float, values: np.ndarray) -> np.ndarray:
    hx = 1.0 / float(nx + 1)
    hy = 1.0 / float(ny + 1)
    ramp, _ = semilinear_ramp(time)
    advection = 0.5 + 3.5 * ramp
    out = np.empty(nx * ny, dtype=np.float64)
    for j in range(ny):
        for i in range(nx):
            index = i + nx * j
            center = float(values[index])
            left = 0.0 if i == 0 else float(values[index - 1])
            right = 0.0 if i + 1 == nx else float(values[index + 1])
            down = 0.0 if j == 0 else float(values[index - nx])
            up = 0.0 if j + 1 == ny else float(values[index + nx])
            laplacian = (left - 2.0 * center + right) / (hx * hx)
            laplacian += (down - 2.0 * center + up) / (hy * hy)
            backward = (center - left) / hx + (center - down) / hy
            out[index] = 0.002 * laplacian - advection * backward - center
    return out


def semilinear_rhs(nx: int, ny: int, time: float, state: np.ndarray) -> np.ndarray:
    phi = semilinear_exact_state(nx, ny, time)
    defect = state - phi
    ramp, _ = semilinear_ramp(time)
    nonlinear = 2.0 + 48.0 * ramp
    return semilinear_operator(nx, ny, time, defect) - phi + nonlinear * (state * state - phi * phi)


def semilinear_jacobian(nx: int, ny: int, time: float, state: np.ndarray) -> csc_matrix:
    hx = 1.0 / float(nx + 1)
    hy = 1.0 / float(ny + 1)
    ramp, _ = semilinear_ramp(time)
    advection = 0.5 + 3.5 * ramp
    nonlinear = 2.0 + 48.0 * ramp
    rows: list[int] = []
    columns: list[int] = []
    values: list[float] = []
    for j in range(ny):
        for i in range(nx):
            index = i + nx * j
            rows.append(index)
            columns.append(index)
            values.append(
                -0.004 / (hx * hx)
                - 0.004 / (hy * hy)
                - advection / hx
                - advection / hy
                - 1.0
                + 2.0 * nonlinear * float(state[index])
            )
            if i > 0:
                rows.append(index)
                columns.append(index - 1)
                values.append(0.002 / (hx * hx) + advection / hx)
            if i + 1 < nx:
                rows.append(index)
                columns.append(index + 1)
                values.append(0.002 / (hx * hx))
            if j > 0:
                rows.append(index)
                columns.append(index - nx)
                values.append(0.002 / (hy * hy) + advection / hy)
            if j + 1 < ny:
                rows.append(index)
                columns.append(index + nx)
                values.append(0.002 / (hy * hy))
    return csc_matrix((values, (rows, columns)), shape=(nx * ny, nx * ny))


def semilinear_runtime(dimension: int):
    nx, ny = semilinear_grid_shape(dimension)
    rhs = lambda time_value, state: semilinear_rhs(nx, ny, time_value, state)
    jac = lambda time_value, state: semilinear_jacobian(nx, ny, time_value, state)
    initial = semilinear_exact_state(nx, ny, 0.0)
    return rhs, jac, initial, nx

FAMILIES = {
    "robertson-ramped": (robertson_runtime, (0.0, 0.1)),
    "hires-ramped": (hires_runtime, (0.0, 1.0)),
    "van-der-pol-ramped": (vdp_runtime, (0.0, 1.0)),
    "rotating-nonnormal": (rotating_runtime, (0.0, 1.0)),
    "nonautonomous-stiff-forcing": (forcing_runtime, (0.0, 1.0)),
    "semilinear-advection-diffusion-ramped": (semilinear_runtime, (0.0, 1.0)),
}
