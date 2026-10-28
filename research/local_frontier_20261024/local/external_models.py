"""Independently expressed, descriptor-verified IVPs shared by external adapters.

Only the four declared models are supported. Names alone never establish a match.
"""
import math

SUPPORTED = (1, 10, 15, 16)

def rhs(contract, t, y):
    case, p = contract['case_id'], contract['parameters']
    if case == 1:
        a,b,c=p['k1']*y[0],p['k2']*y[1]*y[2],p['k3']*y[1]**2
        return [-a+b,a-b-c,c]
    if case == 10:
        if p['B'] != [0,0,1] or p['E'] != [0,0,0] or p['c'] != 1:
            raise ValueError('gyro adapter supports declared uniform magnetic field only')
        g=math.sqrt(1+sum(v*v for v in y[3:]))
        return [y[3]/g,y[4]/g,y[5]/g,p['omega_c']*y[4]/g,-p['omega_c']*y[3]/g,0.]
    if case == 15:
        a=2*p['gamma']-p['beta']
        b=p['omega0']**2*math.exp(2*p['beta']*t)+p['gamma']**2-p['gamma']*p['beta']
        return [y[1],-a*y[1]-b*y[0]]
    if case == 16:
        r2=y[0]*y[0]+y[1]*y[1]
        radial=-p['alpha']/r2**2
        w=p['omega0']/r2**.75
        return [radial*y[0]-w*y[1],radial*y[1]+w*y[0]]
    raise ValueError('external model not implemented; no substitution is permitted')

def verify(contract):
    if contract['case_id'] not in SUPPORTED:
        raise ValueError('unsupported comparator model')
    if contract['t_start'] != 0 or not contract['t_end'] > 0:
        raise ValueError('adapter requires positive span starting at zero')
    initial={1:[1,0,0],10:[0,0,0,1,0,.2],15:[1,-.8],16:[1,0]}[contract['case_id']]
    if contract['initial_state'] != initial:
        raise ValueError('initial state differs from supported contract')
    if len(contract.get('rhs_probes',[])) < 2:
        raise ValueError('two independent state/time RHS probes are required')
    maximum=0.
    for probe in contract['rhs_probes']:
        expected=probe['rhs']; actual=rhs(contract,probe['t'],probe['state'])
        if len(actual) != len(expected): raise ValueError('RHS shape mismatch')
        for x,y in zip(actual,expected):
            d=abs(x-y)/max(1,abs(x),abs(y)); maximum=max(maximum,d)
            if not math.isfinite(d) or d>128*2.220446049250313e-16:
                raise ValueError('RHS probe mismatch; external adapter is inadmissible')
    return {'status':'MATCHED_AT_DECLARED_PROBES','max_scaled_probe_difference':maximum,
            'scope':'Declared parameters, initial state, time span and two arbitrary RHS probes; not a formal code-equivalence proof'}

def exact(contract,t):
    case,p=contract['case_id'],contract['parameters']
    if case==1:return None
    if case==10:
        g=math.sqrt(2.04); phase=p['omega_c']*t/g
        return [math.sin(phase)/p['omega_c'],(math.cos(phase)-1)/p['omega_c'],.2*t/g,math.cos(phase),-math.sin(phase),.2]
    if case==15:
        w=p['omega0']*math.exp(p['beta']*t)
        phase=p['omega0']/p['beta']*math.expm1(p['beta']*t)
        amp=math.exp(-p['gamma']*t)
        return [amp*math.cos(phase),amp*(-p['gamma']*math.cos(phase)-w*math.sin(phase))]
    if case==16:
        if p['r0']!=1 or p['alpha']!=.1: raise ValueError('inspiral exact formula contract mismatch')
        r=(1-4*p['alpha']*t)**.25
        phase=p['omega0']*(1-r**2.5)/(2.5*p['alpha'])
        return [r*math.cos(phase),r*math.sin(phase)]
