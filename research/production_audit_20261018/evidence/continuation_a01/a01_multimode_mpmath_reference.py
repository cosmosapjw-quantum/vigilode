import mpmath as mp
mp.mp.dps=100
def f64(x):
 n,d=float(x).as_integer_ratio(); return mp.mpf(n)/d
xi=[f64(-0.4),f64(0.7)]; w=[f64(0.35),f64(0.65)]
z=[mp.mpc(f64(0.3),f64(0.1)),mp.mpc(f64(-0.2),f64(0.25))]; width=f64(0.2); chi=f64(0.1)
def f(a):
 m=sum(wi*ai for wi,ai in zip(w,a)); g=1j*chi*abs(m)**2*m
 return [1j*width*x*ai+g for x,ai in zip(xi,a)]
def rk4(n):
 h=f64(0.01)/n; a=z[:]; rows=[]
 for q in range(n+1):
  if q in (n//2,n): rows.append((q*h,a[:]))
  if q==n: break
  k1=f(a); k2=f([x+h*y/2 for x,y in zip(a,k1)]); k3=f([x+h*y/2 for x,y in zip(a,k2)]); k4=f([x+h*y for x,y in zip(a,k3)])
  a=[x+h*(u+2*v+2*r+s)/6 for x,u,v,r,s in zip(a,k1,k2,k3,k4)]
 return rows
for n in (512,1024):
 print('N',n)
 for t,a in rk4(n): print(mp.nstr(t,30),*[mp.nstr(v,40) for v in a])
