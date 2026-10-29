import json, math
cases=[]
def add(cid, family, model, n, params, y0, t1, exact, rtol=1e-6, atol=1e-9):
    times=[t1*i/40 for i in range(41)]
    c={"case_id":cid,"family":family,"model":model,"parameters":params,"initial_state":y0,"t_span":[0.0,t1],"output_times":times,"rtol":rtol,"atol":[atol]*len(y0),"exact_kind":exact}
    cases.append(c)
# A: exact exponential / logistic
for n in (2,8,32,128,512):
    rates=[-0.1-0.013*i for i in range(n)]
    add(f"A-decay-n{n}","nonstiff_smooth","diagonal_stiff",n,{"rates":rates},[1.0/(i+1) for i in range(n)],2.0,"diagonal_exponential")
for rtol in (1e-4,1e-6,1e-8): add(f"A-logistic-r{rtol:g}","nonstiff_smooth","logistic",1,{"rate":2.0,"capacity":3.0},[0.2],2.0,"logistic",rtol,1e-10)
# B stiff linear and nonlinear kinetics
for n,lam in ((8,1e2),(32,1e3),(128,1e4),(512,1e3)):
    rates=[-1.0 if i%2==0 else -lam*(1+i/(n or 1)) for i in range(n)]
    add(f"B-stiffdiag-n{n}-L{lam:g}","stiff_dissipative","diagonal_stiff",n,{"rates":rates},[1.0 if i%2==0 else 1e-3 for i in range(n)],1.0,"diagonal_exponential",1e-6,1e-10)
add("B-robertson-short","stiff_dissipative","robertson",3,{"k1":0.04,"k2":1e4,"k3":3e7},[1.0,0.0,0.0],0.1,"numerical_reference",1e-6,1e-12)
add("B-vdp-mu20","stiff_dissipative","vanderpol",2,{"mu":20.0},[2.0,0.0],2.0,"numerical_reference",1e-6,1e-9)
# C oscillatory exact rotation, blocks represented diagonal? Rotation exact 2D
for w in (10.0,100.0): add(f"C-rotation-w{w:g}","rapid_oscillating","rotation",2,{"omega":w},[1.0,0.0],2*math.pi,"rotation",1e-6,1e-10)
# D under/critical/over damping, frequency 8
for z in (0.2,1.0,2.0): add(f"D-damping-z{z:g}","damping_oscillation","damping",2,{"omega":8.0,"zeta":z},[1.0,0.0],2.0,"damped_oscillator",1e-6,1e-10)
# E nonnormal triangular exact 2x2
add("E-nonnormal-transient","nonnormal_growth","nonnormal",2,{"a":-1.0,"b":100.0,"d":-1.2},[1.0,1.0],3.0,"nonnormal_triangular",1e-7,1e-10)
# F structured periodic heat with known Fourier mode solution
for n in (32,128,512):
    y=[math.sin(2*math.pi*i/n) for i in range(n)]
    add(f"F-heat-n{n}","structured_large","periodic_heat",n,{"diffusion":0.01},y,1.0,"periodic_heat_fourier",1e-6,1e-9)
# G near zero mixed scales supported diagonal
add("G-mixed-nearzero","robustness","diagonal_stiff",8,{"rates":[-0.2,-1,-3,-10,-30,-100,-300,-1000]},[1,1e-2,1e-4,1e-6,1e-8,1e-10,1e-12,1e-14],1.0,"diagonal_exponential",1e-8,1e-14)
with open("/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z/harness/cases.json","w") as f: json.dump(cases,f,allow_nan=False,indent=2)
print(len(cases))
