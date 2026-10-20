! ISO_C_BINDING bridge from vigilode-bench to the unmodified pinned Fortran
! solvers: Netlib DVODE (MF 21 dense, MF 24 banded) and Hairer RADAU5 and
! RODAS from the Assimulo 3.0 redistribution. No numerical algorithm edits.
!
! The right-hand side and the Jacobian are the shared Rust model, reached
! through C function pointers. Grid outputs come from each solver's own
! output mechanism: DVODE ITASK = 1 (interpolated output at TOUT), RADAU5
! SOLOUT + CONTR5, RODAS SOLOUT + CONTRO.
!
! Serial use only: DVODE and the Hairer codes keep COMMON state.
module bench_bridge
  use iso_c_binding
  use, intrinsic :: ieee_arithmetic
  implicit none
  abstract interface
    subroutine rhs_cb(ctx, t, y, f) bind(C)
      import c_ptr, c_double
      type(c_ptr), value :: ctx
      real(c_double), value :: t
      real(c_double), intent(in) :: y(*)
      real(c_double), intent(out) :: f(*)
    end subroutine
    subroutine jac_cb(ctx, t, y, ml, mu, ld, pd) bind(C)
      import c_ptr, c_double, c_int
      type(c_ptr), value :: ctx
      real(c_double), value :: t
      real(c_double), intent(in) :: y(*)
      integer(c_int), value :: ml, mu, ld
      real(c_double), intent(out) :: pd(*)
    end subroutine
  end interface
  procedure(rhs_cb), pointer :: p_rhs => null()
  procedure(jac_cb), pointer :: p_jac => null()
  type(c_ptr) :: g_ctx = c_null_ptr
  integer :: g_banded = 0, g_ml = 0, g_mu = 0
  integer :: g_n = 0, g_nt = 0, g_next = 0
  real(c_double), pointer :: g_times(:) => null()
  real(c_double), pointer :: g_values(:,:) => null()
contains

  subroutine bind_problem(ctx, rhs, jac, n, nt, times, values, ml, mu)
    type(c_ptr), value :: ctx, times, values
    type(c_funptr), value :: rhs, jac
    integer(c_int), value :: n, nt, ml, mu
    g_ctx = ctx
    call c_f_procpointer(rhs, p_rhs)
    call c_f_procpointer(jac, p_jac)
    g_n = n; g_nt = nt
    call c_f_pointer(times, g_times, [nt])
    call c_f_pointer(values, g_values, [n, nt])
    g_values = ieee_value(0.0_c_double, ieee_quiet_nan)
    if (ml < 0) then
      g_banded = 0; g_ml = n; g_mu = n
    else
      g_banded = 1; g_ml = ml; g_mu = mu
    end if
  end subroutine

  ! ---------------- shared callbacks in each solver's calling convention
  subroutine fcn(n, x, y, f, rpar, ipar)
    integer :: n, ipar(*)
    double precision :: x, y(n), f(n), rpar(*)
    call p_rhs(g_ctx, x, y, f)
  end subroutine

  ! DVODE: JAC(NEQ, T, Y, ML, MU, PD, NROWPD, RPAR, IPAR); band PD(i-j+MU+1, j).
  subroutine vjac(n, t, y, ml, mu, pd, nrowpd, rpar, ipar)
    integer :: n, ml, mu, nrowpd, ipar(*)
    double precision :: t, y(n), pd(nrowpd, *), rpar(*)
    if (g_banded == 0) then
      call p_jac(g_ctx, t, y, -1_c_int, -1_c_int, int(nrowpd, c_int), pd)
    else
      call p_jac(g_ctx, t, y, int(ml, c_int), int(mu, c_int), int(nrowpd, c_int), pd)
    end if
  end subroutine

  ! Hairer: JAC(N, X, Y, DFY, LDFY, RPAR, IPAR); band DFY(i-j+MUJAC+1, j).
  subroutine hjac(n, x, y, dfy, ldfy, rpar, ipar)
    integer :: n, ldfy, ipar(*)
    double precision :: x, y(n), dfy(ldfy, *), rpar(*)
    if (g_banded == 0) then
      call p_jac(g_ctx, x, y, -1_c_int, -1_c_int, int(ldfy, c_int), dfy)
    else
      call p_jac(g_ctx, x, y, int(g_ml, c_int), int(g_mu, c_int), int(ldfy, c_int), dfy)
    end if
  end subroutine

  subroutine dummy_mas(n, am, lmas, rpar, ipar)
    integer :: n, lmas, ipar(*)
    double precision :: am(lmas, *), rpar(*)
  end subroutine

  subroutine dummy_dfx(n, x, y, fx, rpar, ipar)
    integer :: n, ipar(*)
    double precision :: x, y(n), fx(n), rpar(*)
  end subroutine

  ! Assimulo 3.0 RADAU5 SOLOUT: (NR, XOLD, X, Y, CONT, WERR, LRC, N, RPAR, IPAR, IRTRN).
  subroutine solout_radau(nr, xold, x, y, cont, werr, lrc, n, rpar, ipar, irtrn)
    integer :: nr, lrc, n, ipar(*), irtrn, i
    double precision :: xold, x, y(n), cont(lrc), werr(n), rpar(*), tk
    double precision, external :: contr5
    do while (g_next <= g_nt)
      tk = g_times(g_next)
      if (tk > x) exit
      if (tk == x) then
        g_values(:, g_next) = y
      else
        do i = 1, n
          g_values(i, g_next) = contr5(i, tk, cont, lrc)
        end do
      end if
      g_next = g_next + 1
    end do
  end subroutine

  ! RODAS SOLOUT: (NR, XOLD, X, Y, CONT, LRC, N, RPAR, IPAR, IRTRN).
  subroutine solout_rodas(nr, xold, x, y, cont, lrc, n, rpar, ipar, irtrn)
    integer :: nr, lrc, n, ipar(*), irtrn, i
    double precision :: xold, x, y(n), cont(lrc), rpar(*), tk
    double precision, external :: contro
    do while (g_next <= g_nt)
      tk = g_times(g_next)
      if (tk > x) exit
      if (tk == x) then
        g_values(:, g_next) = y
      else
        do i = 1, n
          g_values(i, g_next) = contro(i, tk, cont, lrc)
        end do
      end if
      g_next = g_next + 1
    end do
  end subroutine

  ! ---------------- DVODE: vendor defaults (IOPT = 0), scalar tolerances.
  subroutine bench_dvode(ctx, rhs, jac, n, nt, times, y0, rtol, atol, ml, mu, &
      values, completed, code, stats) bind(C, name='bench_dvode')
    type(c_ptr), value :: ctx, times, values
    type(c_funptr), value :: rhs, jac
    integer(c_int), value :: n, nt, ml, mu
    real(c_double), value :: rtol, atol
    real(c_double), intent(in) :: y0(n)
    integer(c_int), intent(out) :: completed, code, stats(8)
    double precision, allocatable :: rw(:), y(:)
    integer, allocatable :: iw(:)
    double precision :: t, tout, rt(1), at(1), rpar(1)
    integer :: lrw, liw, mf, istate, ipar(1), k
    external dvode, xsetf
    stats = 0; completed = 0; code = -99
    if (n < 1 .or. nt < 2) return
    call bind_problem(ctx, rhs, jac, n, nt, times, values, ml, mu)
    if (ml < 0) then
      mf = 21
      lrw = 22 + 9*n + 2*n*n
    else
      mf = 24
      lrw = 22 + 11*n + (3*ml + 2*mu)*n
    end if
    liw = 30 + n
    allocate(rw(lrw), iw(liw), y(n))
    rw = 0.0d0; iw = 0; y = y0
    if (ml >= 0) then
      iw(1) = ml; iw(2) = mu
    end if
    rt(1) = rtol; at(1) = atol
    istate = 1; t = g_times(1); rpar = 0.0d0; ipar = 0
    g_values(:, 1) = y; completed = 1
    call xsetf(0)
    do k = 2, nt
      tout = g_times(k)
      call dvode(fcn, n, y, t, tout, 1, rt, at, 1, istate, 0, rw, lrw, iw, liw, vjac, mf, rpar, ipar)
      if (istate < 0) exit
      g_values(:, k) = y
      completed = k
    end do
    code = istate
    ! NST, NFE, NJE, NLU, NNI, NCFN, NETF, NQU
    stats = [iw(11), iw(12), iw(13), iw(19), iw(20), iw(21), iw(22), iw(14)]
    deallocate(rw, iw, y)
  end subroutine

  ! ---------------- Hairer RADAU5 / RODAS: vendor defaults (WORK, IWORK(1:20) = 0),
  ! analytic Jacobian, identity mass, scalar tolerances, initial H = 0 (code default).
  subroutine bench_hairer(which, ctx, rhs, jac, n, nt, times, y0, rtol, atol, ml, mu, &
      values, completed, code, stats) bind(C, name='bench_hairer')
    integer(c_int), value :: which
    type(c_ptr), value :: ctx, times, values
    type(c_funptr), value :: rhs, jac
    integer(c_int), value :: n, nt, ml, mu
    real(c_double), value :: rtol, atol
    real(c_double), intent(in) :: y0(n)
    integer(c_int), intent(out) :: completed, code, stats(7)
    double precision, allocatable :: work(:), y(:)
    integer, allocatable :: iwork(:)
    double precision :: x, xend, h, rt, at, rpar(1)
    integer :: lwork, liwork, ljac, le, mljac, mujac, idid, ipar(1)
    external radau5, rodas
    stats = 0; completed = 0; code = -99
    if (n < 1 .or. nt < 2) return
    call bind_problem(ctx, rhs, jac, n, nt, times, values, ml, mu)
    if (ml < 0) then
      mljac = n; mujac = n; ljac = n; le = n
    else
      mljac = ml; mujac = mu; ljac = ml + mu + 1; le = 2*ml + mu + 1
    end if
    if (which == 0) then
      lwork = n*(ljac + 3*le + 12) + 20
      liwork = 3*n + 20
    else
      lwork = n*(ljac + le + 14) + 20
      liwork = n + 20
    end if
    allocate(work(lwork), iwork(liwork), y(n))
    work = 0.0d0; iwork = 0; y = y0
    x = g_times(1); xend = g_times(nt); h = 0.0d0
    rt = rtol; at = atol; rpar = 0.0d0; ipar = 0; idid = 0
    g_next = 1
    if (which == 0) then
      call radau5(n, fcn, x, y, xend, h, rt, at, 0, hjac, 1, mljac, mujac, &
          dummy_mas, 0, n, n, solout_radau, 1, work, lwork, iwork, liwork, rpar, ipar, idid)
    else
      call rodas(n, fcn, 0, x, y, xend, h, rt, at, 0, hjac, 1, mljac, mujac, &
          dummy_dfx, 0, dummy_mas, 0, n, n, solout_rodas, 1, work, lwork, iwork, liwork, &
          rpar, ipar, idid)
    end if
    ! A successful run ends at X = XEND with Y there; SOLOUT has consumed the
    ! final grid point unless X differs from XEND by rounding.
    if (idid > 0 .and. g_next == nt .and. x == xend) then
      g_values(:, nt) = y
      g_next = nt + 1
    end if
    completed = g_next - 1
    code = idid
    ! NFCN, NJAC, NSTEP, NACCPT, NREJCT, NDEC, NSOL
    stats = iwork(14:20)
    deallocate(work, iwork, y)
  end subroutine
end module
