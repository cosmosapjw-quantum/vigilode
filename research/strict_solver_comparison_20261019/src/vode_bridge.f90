! ISO_C_BINDING bridge to unmodified Netlib DVODE. No numerical algorithm edits.
! Serial use only: original DVODE contains COMMON state and is not reentrant.
module strict_vode_bridge
  use iso_c_binding
  use, intrinsic :: ieee_arithmetic
  implicit none
  interface
    subroutine shared_rhs(id,n,t,y,f) bind(C,name='strict_rhs')
      import c_int,c_double
      integer(c_int),value :: id,n
      real(c_double),value :: t
      real(c_double),intent(in) :: y(n)
      real(c_double),intent(out) :: f(n)
    end subroutine
    subroutine shared_jac(id,n,ml,mu,ld,t,y,j) bind(C,name='strict_jac')
      import c_int,c_double
      integer(c_int),value :: id,n,ml,mu,ld
      real(c_double),value :: t
      real(c_double),intent(in) :: y(n)
      real(c_double),intent(out) :: j(ld,n)
    end subroutine
  end interface
contains
  subroutine vf(n,t,y,f,rpar,ipar)
    integer,intent(in) :: n,ipar(*)
    double precision,intent(in) :: t,y(n),rpar(*)
    double precision,intent(out) :: f(n)
    call shared_rhs(ipar(1),n,t,y,f)
  end subroutine
  subroutine vj(n,t,y,ml,mu,pd,ld,rpar,ipar)
    integer,intent(in) :: n,ml,mu,ld,ipar(*)
    double precision,intent(in) :: t,y(n),rpar(*)
    double precision,intent(out) :: pd(ld,n)
    if (ipar(2) < 0) then
      call shared_jac(ipar(1),n,-1,-1,ld,t,y,pd)
    else
      call shared_jac(ipar(1),n,ml,mu,ld,t,y,pd)
    end if
  end subroutine

  subroutine integrate(id,n,nt,tf,y0,rtol,atol,h0,times,band,landing,values,reached,completed,code,stats) &
      bind(C,name='strict_vode')
    integer(c_int),value :: id,n,nt,band,landing
    real(c_double),value :: tf,rtol,atol,h0
    real(c_double),intent(in) :: y0(n),times(nt)
    real(c_double),intent(out) :: values(n,nt),reached
    integer(c_int),intent(out) :: completed,code,stats(8)
    double precision,allocatable :: rw(:),y(:)
    integer,allocatable :: iw(:)
    double precision :: t,tout,rt(1),at(1),rpar(1)
    integer :: lrw,liw,mf,itask,istate,iopt,itol,ipar(2),k
    external dvode,xsetf
    values=ieee_value(0.0_c_double,ieee_quiet_nan)
    stats=0; completed=0; reached=0.0_c_double; code=-3
    if (n<1 .or. nt<2 .or. band>=n) return
    if (band<0) then
      mf=21
      lrw=22+9*n+2*n*n
    else
      mf=24
      lrw=22+11*n+5*band*n
    end if
    liw=30+n
    allocate(rw(lrw),iw(liw),y(n))
    rw=0.0d0; iw=0; y=y0
    if (band>=0) then
      iw(1)=band; iw(2)=band
    end if
    rw(5)=h0; rw(6)=tf; rw(7)=1.0d-14
    iw(5)=5; iw(6)=1000000
    itol=1; rt(1)=rtol; at(1)=atol
    istate=1; iopt=1; t=times(1); rpar=0.0d0; ipar=[id,band]
    values(:,1)=y; completed=1
    call xsetf(0)
    do k=2,nt
      tout=times(k)
      itask=1
      if (landing/=0) then
        itask=4
        rw(1)=tout
      end if
      call dvode(vf,n,y,t,tout,itol,rt,at,itask,istate,iopt,rw,lrw,iw,liw,vj,mf,rpar,ipar)
      if (istate<0) exit
      if (t/=tout) exit
      values(:,k)=y
      completed=k
    end do
    code=istate; reached=t
    stats=[iw(11),iw(12),iw(13),iw(19),iw(20),iw(21),iw(22),iw(14)]
    deallocate(rw,iw,y)
  end subroutine
end module
