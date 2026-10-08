"""Arm registry for probe A1."""
import rep, coupled_target as CT
STALL = CT.RHO_STALL
def arm(name):
    """returns (arm dict, Target or None)"""
    if name == 'base': return rep.make_arm('base'), None
    if name == 'lu': return rep.make_arm('lu'), None
    if name == 'proj': return rep.make_arm('proj'), None                               # SPD07 L2 1e-10, floor g*1e-14
    if name == 'proj12': return rep.make_arm('proj', rtol_lin=1e-12), None
    if name == 'proj12nf': return rep.make_arm('proj', rtol_lin=1e-12, atol_lin=1e-18), None   # judge: floor removed
    if name == 'proj13nf': return rep.make_arm('proj', rtol_lin=1e-13, atol_lin=1e-18), None
    if name == 'l2c': return rep.make_arm('l2c', cr=1e-3, ca=1e-3), None                  # judge: rtol_lin=min(1e-10,1e-3 rtol), atol_lin=1e-3 atol
    if name == 'l2c4': return rep.make_arm('l2c', cr=1e-4, ca=1e-4), None
    if name == 'ino':      # INO-FORCE-ABS uncertified as run by the judge (theta 1e-3 per step, (err/0.5)^1.2, e0 1e-6, U8 cap, 1e-14 rel floor)
        return rep.make_arm('wabs', stall=STALL), CT.Target(Theta=1e-3, epus=False, rho_fl=1e-14, e_ref=0.5)
    if name.startswith('wS'):   # per-step budget theta (no EPUS), e.g. wS1e-4
        return rep.make_arm('wabs', stall=STALL), CT.Target(Theta=float(name[2:]), epus=False, e_ref=0.5)
    if name.startswith('wE'):   # EPUS budget Theta, e.g. wE0.02 ; trailing letter flags in any order
        base = name[2:]; kw = dict(e_ref=0.5); form = 'scaled'; nostall = False
        while base and base[-1].isalpha():
            fl = base[-1]; base = base[:-1]
            if fl == 's': nostall = True               # no stall rule (control)
            elif fl == 'L': form = 'l2test'            # L2 GMRES, safe max(D) conversion
            elif fl == 'R': form = 'l2rms'             # L2 GMRES, rms(D) conversion + continuation
            elif fl == 'r': kw['e_ref'] = 0.1          # order factor referenced to 0.1
            elif fl == 'n': kw['p'] = 0                # no order factor (control)
            elif fl == 'u': kw['alloc'] = 'uniform'    # uniform allocation theta/sum(tau_y)
            elif fl == 'c': kw['kappa8'] = None        # no U8 cap (control)
            else: raise ValueError(name)
        return rep.make_arm('wabs', form=form, stall=(None if nostall else STALL)), CT.Target(Theta=float(base), epus=True, **kw)
    raise ValueError(name)
