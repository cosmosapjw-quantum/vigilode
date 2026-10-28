#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cfenv>
#if defined(__SSE__)
#include <xmmintrin.h>
#endif
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include <map>
#include <stdexcept>
#include <string>
#include <vector>
using V=std::vector<double>;
// All primitive interval operations round outward by one binary64 successor.
// Build requires IEEE binary64 round-to-nearest, no FTZ/DAZ, no fast-math or FMA contraction.
static double dn(double x){return std::nextafter(x,-INFINITY);}
static double up(double x){return std::nextafter(x,INFINITY);}
struct I {
 double l,h;
 I(double a=0):l(a),h(a){} I(double a,double b):l(a),h(b){if(!(a<=b))throw std::runtime_error("invalid interval");}
};
I operator+(I a,I b){return {dn(a.l+b.l),up(a.h+b.h)};}
I operator-(I a,I b){return {dn(a.l-b.h),up(a.h-b.l)};}
I operator-(I a){return {-a.h,-a.l};}
I operator*(I a,I b){double x[4]={a.l*b.l,a.l*b.h,a.h*b.l,a.h*b.h};return {dn(*std::min_element(x,x+4)),up(*std::max_element(x,x+4))};}
I operator/(I a,I b){if(b.l<=0&&b.h>=0)throw std::runtime_error("interval denominator zero");return a*I(dn(1/b.h),up(1/b.l));}
static double mag(I a){return std::max(std::abs(a.l),std::abs(a.h));}
#include "circuits.hpp"
template<class T> std::vector<T> structured_solve(const std::vector<T>&q,const std::vector<T>&f){
 int n=q.size();std::vector<T>b(n),v(n);for(int i=0;i<n;i++)b[i]=q[(i+1)%n]*T(.5);
 T r=f[n-1],p=-b[n-1];for(int i=n-2;i>=0;i--){r=f[i]-b[i]*r;p=-b[i]*p;}
 v[0]=r/(T(1)-p);v[n-1]=f[n-1]-b[n-1]*v[0];for(int i=n-2;i>=1;i--)v[i]=f[i]-b[i]*v[i+1];return v;
}
template<class T> std::vector<T> slow(const std::vector<T>&x,const std::vector<T>&q){int n=x.size();std::vector<T>f(n);for(int i=0;i<n;i++)f[i]=-x[i]+(q[i]+q[(i+1)%n])*T(1./16)+T(1./32+(i%3)/256.);return f;}
template<class T> std::vector<T> slope(const std::vector<T>&y){int n=y.size()/2;std::vector<T>x(y.begin(),y.begin()+n),q(y.begin()+n,y.end());auto f=slow(x,q);auto v=compiled_solve(q,f);f.insert(f.end(),v.begin(),v.end());return f;}
using P=std::vector<I>;
static P add(P a,const P&b){a.resize(std::max(a.size(),b.size()));for(size_t i=0;i<b.size();i++)a[i]=a[i]+b[i];return a;}
static P scale(P a,I b){for(auto&x:a)x=x*b;return a;}
static P mul(const P&a,const P&b){P r(a.size()+b.size()-1);for(size_t i=0;i<a.size();i++)for(size_t j=0;j<b.size();j++)r[i+j]=r[i+j]+a[i]*b[j];return r;}
static P deriv(const P&a){P r;for(size_t i=1;i<a.size();i++)r.push_back(a[i]*I(i));return r;}
static int choose(int n,int k){int c=1;for(int j=1;j<=k;j++)c=c*(n-j+1)/j;return c;}
static I range(const P&a){int n=a.size()-1;double l=INFINITY,h=-INFINITY;for(int i=0;i<=n;i++){I c;for(int k=0;k<=i;k++)c=c+a[k]*(I(choose(i,k))/I(choose(n,k)));l=std::min(l,c.l);h=std::max(h,c.h);}return {l,h};}
struct Cert {double dg=0,df=0,ew=0,ee=0,qw=0,e0=0;};
static Cert certificate(const V&y0,const V&y1,double h,double e){
 int n=y0.size()/2;std::vector<I>a(y0.begin(),y0.end()),b(y1.begin(),y1.end());auto f0=slope(a),f1=slope(b);std::vector<P>path(2*n);
 for(int i=0;i<2*n;i++)path[i]={a[i],I(h)*f0[i],I(3)*(b[i]-a[i])-I(h)*(I(2)*f0[i]+f1[i]),I(2)*(a[i]-b[i])+I(h)*(f0[i]+f1[i])};
 Cert c;c.e0=e;for(int i=0;i<2*n;i++){auto rr=range(path[i]);double limit=i<n?.125:.25;if(mag(rr)>limit)throw std::runtime_error("path domain failure");}
 for(int i=0;i<n;i++){
  int j=(i+1)%n;
  P g=add(add(path[n+i],scale(mul(path[n+j],path[n+j]),I(.25))),scale(path[i],I(-1)));
  P ff=add(add(scale(path[i],I(-1)),scale(add(path[n+i],path[n+j]),I(1./16))),P{I(1./32+(i%3)/256.)});
  P d=add(scale(deriv(path[i]),I(1)/I(h)),scale(ff,I(-1)));
  c.dg=std::max(c.dg,mag(range(g)));c.df=std::max(c.df,mag(range(d)));
 }
 I delta=I(c.df)+I(c.dg)/I(7);c.ew=(I(e)+I(h)*delta).h;c.ee=(I(e)/(I(1)+I(h)*(I(6)/I(7)))+I(h)*delta).h;c.qw=((I(c.ew)+I(c.dg))*(I(8)/I(7))).h;return c;
}
struct Counts {long rhs=0,roots=0,updates=0,cache_hits=0,kraw=0,kraw_fail=0,structured=0;double kraw_seconds=0;};
// Sparse-J Krawczyk inclusion with a binary64 preconditioner R; all inclusion arithmetic interval.
static bool krawczyk(const V&x,const V&q,Counts&c){
 int n=q.size();double rad=std::ldexp(1.,-42);std::vector<V>R(n,V(n));
 for(int j=0;j<n;j++){V e(n);e[j]=1;auto col=structured_solve(q,e);c.structured++;for(int i=0;i<n;i++)R[i][j]=col[i];}
 std::vector<I>g(n);for(int j=0;j<n;j++){int k=(j+1)%n;g[j]=I(q[j])+I(.25)*I(q[k])*I(q[k])-I(x[j]);if((I(std::abs(q[j]))+I(rad)).h>=.25)return false;}
 for(int i=0;i<n;i++){
  I off;for(int j=0;j<n;j++)off=off-I(R[i][j])*g[j];
  I rr;for(int j=0;j<n;j++){int prev=(j+n-1)%n;I b=I(.5)*I(dn(q[j]-rad),up(q[j]+rad));I m=I(i==j?1.:0.)-I(R[i][j])-I(R[i][prev])*b;rr=rr+I(mag(m))*I(rad);}
  if((I(mag(off))+rr).h>=rad)return false;
 }
 c.kraw++;return true;
}
struct Integrator {
 int n,method;Counts c;V last;std::map<V,V>cache;
 Integrator(int N,int M):n(N),method(M),last(N){cache[V(n)]=V(n);}
 V root(const V&x){auto it=cache.find(x);if(it!=cache.end()){c.cache_hits++;return it->second;}c.roots++;V q=last;
  for(int k=0;k<12;k++){
   V g(n);double m=0;for(int i=0;i<n;i++){g[i]=q[i]+.25*q[(i+1)%n]*q[(i+1)%n]-x[i];m=std::max(m,std::abs(g[i]));}
   if(m<=2e-16){if(method==2){auto kt=std::chrono::steady_clock::now();bool ok=krawczyk(x,q,c);c.kraw_seconds+=std::chrono::duration<double>(std::chrono::steady_clock::now()-kt).count();if(!ok){c.kraw_fail++;throw std::runtime_error("Krawczyk inclusion failed");}}last=q;cache[x]=q;return q;}
   auto d=structured_solve(q,g);c.structured++;c.updates++;for(int i=0;i<n;i++){q[i]-=d[i];if(!std::isfinite(q[i]))throw std::runtime_error("Newton nonfinite");}
  }throw std::runtime_error("Newton iterations exhausted");
 }
 V rhs(const V&y){c.rhs++;if(method==0)return slope(y);return slow(y,root(y));}
 std::vector<V> run(int N){double h=1./N;V y(method==0?2*n:n);std::vector<V>states;states.push_back(V(2*n));
  for(int s=0;s<N;s++){auto k1=rhs(y);V tmp(y.size());for(size_t i=0;i<y.size();i++)tmp[i]=y[i]+h*k1[i]/2;auto k2=rhs(tmp);for(size_t i=0;i<y.size();i++)tmp[i]=y[i]+h*k2[i]/2;auto k3=rhs(tmp);for(size_t i=0;i<y.size();i++)tmp[i]=y[i]+h*k3[i];auto k4=rhs(tmp);for(size_t i=0;i<y.size();i++)y[i]+=h*(k1[i]+2*k2[i]+2*k3[i]+k4[i])/6;V full=y;if(method){auto q=root(y);full.insert(full.end(),q.begin(),q.end());}states.push_back(full);}
 return states;
 }
};
static double elapsed(std::chrono::steady_clock::time_point a){return std::chrono::duration<double>(std::chrono::steady_clock::now()-a).count();}
static void vec(const V&a){std::cout<<"[";for(size_t i=0;i<a.size();i++){if(i)std::cout<<",";std::cout<<a[i];}std::cout<<"]";}
int main(int argc,char**argv){try{
 if(argc!=5)throw std::runtime_error("args: n steps method repeats");int n=std::stoi(argv[1]),N=std::stoi(argv[2]),m=std::stoi(argv[3]),reps=std::stoi(argv[4]);
 if((n!=4&&n!=8&&n!=16)||N<1||(N&(N-1))||m<0||m>2||reps<1)throw std::runtime_error("invalid args");
 if(std::numeric_limits<double>::is_iec559==false||std::numeric_limits<double>::digits!=53)throw std::runtime_error("IEEE binary64 required");
 if(std::fegetround()!=FE_TONEAREST)throw std::runtime_error("round-to-nearest required");
#if defined(__SSE__)
 if(_mm_getcsr()&((1u<<15)|(1u<<6)))throw std::runtime_error("FTZ/DAZ must be disabled");
#endif
 std::cout<<std::setprecision(17);V ti,tc,tt,tk;Counts counts;double ew=0,qw=0,df=0,dg=0;std::vector<V>states;std::vector<Cert>cs;
 for(int rep=0;rep<reps;rep++){auto total=std::chrono::steady_clock::now();Integrator integrator(n,m);auto start=std::chrono::steady_clock::now();auto st=integrator.run(N);double integration=elapsed(start);start=std::chrono::steady_clock::now();double e=0;ew=qw=df=dg=0;std::vector<Cert>sc;
 for(int k=0;k<N;k++){auto z=certificate(st[k],st[k+1],1./N,e);e=z.ee;ew=std::max(ew,z.ew);qw=std::max(qw,z.qw);df=std::max(df,z.df);dg=std::max(dg,z.dg);sc.push_back(z);}double cert=elapsed(start);ti.push_back(integration);tc.push_back(cert);tt.push_back(elapsed(total));counts=integrator.c;tk.push_back(counts.kraw_seconds);states=std::move(st);cs=std::move(sc);}
 std::cout<<"{\"n\":"<<n<<",\"steps\":"<<N<<",\"method\":\""<<(m==0?"compiled":m==1?"newton":"krawczyk")<<"\",\"repeats\":"<<reps<<",\"integration_seconds\":";vec(ti);std::cout<<",\"certificate_seconds\":";vec(tc);std::cout<<",\"total_seconds\":";vec(tt);std::cout<<",\"closure_krawczyk_seconds\":";vec(tk);
 std::cout<<",\"slow_whole_bound\":"<<ew<<",\"root_whole_bound\":"<<qw<<",\"ode_residual\":"<<df<<",\"algebraic_residual\":"<<dg<<",\"physical_budget_met\":"<<(std::max(ew,qw)<=1e-8?"true":"false")<<",\"counts\":{\"rhs\":"<<counts.rhs<<",\"root_solves\":"<<counts.roots<<",\"newton_updates\":"<<counts.updates<<",\"cache_hits\":"<<counts.cache_hits<<",\"structured_solves\":"<<counts.structured<<",\"krawczyk_inclusions\":"<<counts.kraw<<",\"krawczyk_failures\":"<<counts.kraw_fail<<",\"generic_lu\":0,\"jvp\":0,\"vjp\":0},\"representative_slabs\":[";
 for(int ix=0;ix<3;ix++){int k=ix==0?0:ix==1?N/2:N-1;auto z=cs[k];if(ix)std::cout<<",";std::cout<<"{\"index\":"<<k<<",\"y0\":";vec(states[k]);std::cout<<",\"y1\":";vec(states[k+1]);std::cout<<",\"e0\":"<<z.e0<<",\"dg\":"<<z.dg<<",\"df\":"<<z.df<<",\"ew\":"<<z.ew<<",\"ee\":"<<z.ee<<",\"qw\":"<<z.qw<<"}";}
 std::cout<<"]}\n";return 0;
 }catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
