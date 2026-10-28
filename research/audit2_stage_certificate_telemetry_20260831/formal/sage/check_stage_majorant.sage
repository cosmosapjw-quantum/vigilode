# Candidate-free exact-rational F02 and F04 checks for FORMAL_SCOPE.md.
R = QQ

# F02: W is the declared two-timescale triangular operator, D its Jacobi map.
W = matrix(R, [[2, 1], [0, 3]])
D = diagonal_matrix(R, [2, 3])
I2 = identity_matrix(R, 2)
assert (D.inverse() * W - I2)^2 == zero_matrix(R, 2)
assert (W * D.inverse() - I2)^2 == zero_matrix(R, 2)
print("F02_SAGE_JACOBI_SQUARE_ZERO")

# F04: exact strictly-lower propagation and its nonnegative weighted bounds.
T = matrix(R, [[0, 0, 0], [R(1)/4, 0, 0], [R(1)/5, R(1)/3, 0]])
q = vector(R, [R(1)/10, R(1)/5, R(3)/10])
u = vector(R, [0, 0, 0])
for i in range(3):
    u[i] = q[i] + sum(T[i, j] * u[j] for j in range(i))
assert u == (identity_matrix(R, 3) - T).inverse() * q
assert all(value >= 0 for value in u)
endpoint_weights = vector(R, [R(1)/2, R(1)/3, R(1)/6])
estimator_weights = vector(R, [R(1)/3, R(1)/3, R(1)/3])
assert endpoint_weights.dot_product(u) == R(229)/1200
assert estimator_weights.dot_product(u) == R(6)/25
assert endpoint_weights.dot_product(u) >= 0
assert estimator_weights.dot_product(u) >= 0
print("F04_SAGE_FORWARD_MAJORANT_AND_WEIGHTED_BOUNDS")
print("SAGE_FORMAL_PASS")
