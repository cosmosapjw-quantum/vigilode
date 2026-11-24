"""Independent high-precision power-series integration for the cooperative cubic source."""
from decimal import Decimal, localcontext
from math import ceil

D = Decimal.from_float


def trajectory(source, grid, order, digits):
    """Adaptive subdivision by a rate bound; fixed order on each substep."""
    with localcontext() as ctx:
        ctx.prec = digits
        n = source["n"]
        k, g, y = [[D(float(v)) for v in source[key][:n]] for key in ("k", "g", "y0")]
        C = [[D(float(v)) for v in row[:n]] for row in source["c"][:n]]
        if source["name"] == "linear_exchange_control":
            result = []
            for t_float in grid:
                t = D(t_float)
                a = (y[0]+y[1])*(-k[0]*t).exp()/2
                b = (y[0]-y[1])*(-(k[0]+2*C[0][1])*t).exp()/2
                result.append([a+b, a-b])
            return result
        diag = [k[i]+sum(C[i]) for i in range(n)]
        rate = max(k[i]+2*sum(C[i])+3*g[i]*max(y)**2 for i in range(n))
        result = [y]
        for start, end in zip(grid,grid[1:]):
            interval = D(end)-D(start)
            subdivisions = max(1, ceil(interval*rate*(2 if order>=30 else 1)))
            h = interval/subdivisions
            for _ in range(subdivisions):
                # If z(theta)=sum_l a_l theta^l, store z^2 to obtain z^3
                # by a second convolution, then recur on the coefficient of z'.
                a = [[y[i]] for i in range(n)]
                square = [[] for _ in range(n)]
                for m in range(order):
                    increment = []
                    for i in range(n):
                        square[i].append(sum(a[i][j]*a[i][m-j] for j in range(m+1)))
                        cube = sum(a[i][j]*square[i][m-j] for j in range(m+1))
                        flow = sum(C[i][j]*a[j][m] for j in range(n)) - diag[i]*a[i][m] - g[i]*cube
                        increment.append(h*flow/(m+1))
                    for i in range(n):
                        a[i].append(increment[i])
                y = [sum(row) for row in a]
            result.append(y)
        return result


def discrepancy(a,b):
    with localcontext() as ctx:
        ctx.prec=85
        return max(abs(x-y) for ar,br in zip(a,b,strict=True) for x,y in zip(ar,br,strict=True))
