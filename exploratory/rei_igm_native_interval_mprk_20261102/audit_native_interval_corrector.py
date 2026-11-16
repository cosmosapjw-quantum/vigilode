#!/usr/bin/env python3
"""Independent stdlib-only original REI interval MPRK corrector audit.

Unlike implementation, this reads raw binary and original owner CSV independently.
It never treats corrector-only enclosures as full 26-event thermochemical interval IVP.
"""
from __future__ import annotations
import argparse,csv,gzip,hashlib,json,math,struct
from decimal import Decimal,getcontext
from pathlib import Path

N=46080
COLS=70
SOURCE_SHA="95b2cdbfeb5777c2b893910f1663c271d0ead673629180ece068bf3e0a6fb001"
OWNER_CSV_SHA="11df439390209bd48742ca9f4303c8743c9e14900289805e13d1ed489450a5dd"
METHODS=("full","half1","half2")
CHANNELS={"x_HII":1,"x_HeII":3,"x_HeIII":4}
SAMPLE=(0,1,17,919,12001,25003,36061,46079)

def sha(data):return hashlib.sha256(data).hexdigest()
def source_row(raw,m,i):
    if raw[:8]!=b"REIF05C2" or len(raw)!=24+3*(16+COLS*N*8):
        raise ArithmeticError("ORIGINAL_OWNER_SOURCE_BINARY_INVALID")
    if struct.unpack_from("<QQ",raw,8)!=(N,3):raise AssertionError("ORIGINAL_HEADER_COUNT")
    return struct.unpack_from("<70d",raw,24+m*(16+COLS*N*8)+16+i*COLS*8)
def native_row(raw,m,i):
    if raw[:8]!=b"REIF05I2" or len(raw)!=16+3*N*10*8:
        raise ArithmeticError("RUST_INTERVAL_BINARY_INVALID")
    if struct.unpack_from("<Q",raw,8)!=(N,):raise AssertionError("RUST_HEADER_COUNT")
    x=struct.unpack_from("<10d",raw,16+m*N*10*8+i*10*8)
    return list(zip(x[:5],x[5:]))
def bounds_ratio(low,high,denlow,denhigh):
    if denlow<=0 or denhigh<denlow:raise AssertionError("PHYSICAL_HHE_NUCLEI_DENOM_INVALID")
    x=(low/denlow,low/denhigh,high/denlow,high/denhigh)
    return math.nextafter(min(x),-math.inf),math.nextafter(max(x),math.inf)
def exact_point_trial(source,dt):
    """120-digit Decimal solve for f64-derived source midpoints; not an exact-rational proof."""
    getcontext().prec=120
    D=Decimal.from_float
    p=[(D(source[i])+D(source[5+i]))/2 for i in range(5)]
    den=[(D(source[60+i])+D(source[65+i]))/2 for i in range(5)]
    f=[(max(D(source[10+i]),Decimal(0))+
         max(D(source[35+i]),Decimal(0)))/2 for i in range(25)]
    h=D(dt)
    A=[[Decimal(int(i==j)) for j in range(5)] for i in range(5)]
    for s in range(5):
        for d in range(5):
            if d==s:continue
            r=h*f[5*d+s]/den[s]
            A[s][s]+=r;A[d][s]-=r
    out=[None]*5
    for start,n in ((0,2),(2,3)):
        a=[[A[start+i][start+j] for j in range(n)] for i in range(n)]
        b=p[start:start+n]
        # Gaussian elimination with Decimal and tiny 2/3 physical block.
        for k in range(n):
            if a[k][k]<=0:raise AssertionError("ORIGINAL_M_MATRIX_PIVOT_NOT_POSITIVE")
            for i in range(k+1,n):
                z=a[i][k]/a[k][k]
                for j in range(k+1,n):a[i][j]-=z*a[k][j]
                b[i]-=z*b[k]
                a[i][k]=Decimal(0)
        y=[Decimal(0)]*n
        for i in reversed(range(n)):
            y[i]=(b[i]-sum(a[i][j]*y[j] for j in range(i+1,n)))/a[i][i]
        for i in range(n):out[start+i]=y[i]-p[start+i]
    return out
def audit(source_file,native_file,receipt_file,owner_csv,owner_receipt,outfile):
    source=source_file.read_bytes()
    native=native_file.read_bytes()
    receipt=json.loads(receipt_file.read_text())
    own=json.loads(owner_receipt.read_text())
    if sha(source)!=SOURCE_SHA:raise AssertionError("ORIGINAL_SOURCE_BINARY_SHA_MISMATCH")
    if sha(owner_csv.read_bytes())!=OWNER_CSV_SHA:raise AssertionError("ORIGINAL_OWNER_CSV_SOURCE_CHANGED")
    if sha(native)!=receipt["native_interval_increment_raw_sha256"]:raise AssertionError("RUST_INTERVAL_SHA_MISMATCH")
    if receipt["schema"]!="REI_F05_NATIVE_RUST_ORIGINAL_CORRECTOR_INTERVAL_PROOF_V1":raise AssertionError("RUST_RESULT_SCHEMA_DRIFT")
    if receipt["original_source_binary_sha256"]!=SOURCE_SHA:raise AssertionError("RUST_SOURCE_IDENTITY_DRIFT")
    if own["original_set_ledgers"]["passed"] is not True:raise AssertionError("ORIGINAL_LEDGER_FAILED")
    if own["original_set_ledgers"]["evaluated_ledgers"]!=14:raise AssertionError("ORIGINAL_LEDGER_COUNT_CHANGED")
    if own["original_native_table_event_pass"] is not True:raise AssertionError("ORIGINAL_TABLE_EVENT")
    if own["status"]!="SCOPED_SOURCE_REAL_F05_INCREMENT_GATE_CANDIDATE_PASS":raise AssertionError("OWNER_CANDIDATE_STATUS_DRIFT")
    zero_positive=0
    scan=0
    nonfinite=0
    samples=[]
    electron_summary=[]
    for m in range(3):
        electron_lo_terms=[]
        electron_hi_terms=[]
        for i in range(N):
            r=native_row(native,m,i)
            if any(not(math.isfinite(lo) and math.isfinite(hi) and lo<=hi)
                   for lo,hi in r):
                nonfinite+=1
            src=source_row(source,m,i)
            electron_lo_terms.append(r[1][0]+r[3][0]+2.0*r[4][0])
            electron_hi_terms.append(r[1][1]+r[3][1]+2.0*r[4][1])
            # Independent positive-cone and conservation necessary checks;
            # NOT a proof of the full nonlinear chemical operator.
            for start,n in ((0,2),(2,3)):
                blo=sum(src[j] for j in range(start,start+n))
                bhi=sum(src[j+5] for j in range(start,start+n))
                # Sum delta must include 0. Capture both error signs with
                # intentionally generous ulp on the independent audit only.
                dlo=sum(r[j][0] for j in range(start,start+n))
                dhi=sum(r[j][1] for j in range(start,start+n))
                if dlo>0 or dhi<0:
                    raise AssertionError(("NUCLEI_DELTA_ZERO_EXCLUDED",m,i,start,dlo,dhi))
                for j in range(start,start+n):
                    if src[j]+r[j][1]<=0:
                        raise AssertionError(("EXACT_POSITIVE_CANDIDATE_EXCLUDED",m,i,j))
            if i in SAMPLE:
                dts=(4871848749.168213,2435924374.5841064,2435924374.5841064)
                exact=exact_point_trial(src,dts[m])
                for j,(lo,hi) in enumerate(r):
                    ex=exact[j]
                    if not (Decimal.from_float(lo)<=ex<=Decimal.from_float(hi)):
                        raise AssertionError(("HIGH_PRECISION_TRIAL_NOT_ENCLOSED",m,i,j,str(ex),lo,hi))
                samples.append({"map":METHODS[m],"node":i,"decimal_120digit_components_checked":5})
            scan+=1
        elo=math.fsum(electron_lo_terms)
        ehi=math.fsum(electron_hi_terms)
        certified=receipt["native_interval_corrector_map_report"][m][
            "certified_source_box_total_delta_electrons"]
        if not certified[0]<=elo<=ehi<=certified[1]:
            raise AssertionError(("RUST_OUTWARD_ELECTRON_SUM_DISAGREES_WITH_INDEPENDENT_REDUCTION",m,certified,elo,ehi))
        electron_summary.append(dict(map=METHODS[m],
            rust_verified_interval_electrons=certified,
            independent_fsum_interval_electrons=[elo,ehi],
            negative_sign_certified=certified[1]<0))
    if nonfinite or scan!=3*N:raise AssertionError("NATIVE_INTERVAL_INCORRECT_ROW_COUNT")
    overlap={name:{c:0 for c in CHANNELS} for name in ("full","twohalf")}
    missing=[]
    with gzip.open(owner_csv,"rt",newline="") as fp:
        rd=csv.DictReader(fp)
        for i,row in enumerate(rd):
            if int(row["node"])!=i or i>=N:raise AssertionError("OWNER_INCREMENT_ROW_ORDER")
            source_t160=source_row(source,0,i)
            nhlo=sum(source_t160[j] for j in (0,1))
            nhhi=sum(source_t160[j+5] for j in (0,1))
            helo=sum(source_t160[j] for j in (2,3,4))
            hehi=sum(source_t160[j+5] for j in (2,3,4))
            ds_full=native_row(native,0,i)
            dh1=native_row(native,1,i)
            dh2=native_row(native,2,i)
            ds_half=[(dh1[k][0]+dh2[k][0],dh1[k][1]+dh2[k][1]) for k in range(5)]
            for name,ds,ref in (("full",ds_full,"original_full"),
                                ("twohalf",ds_half,"original_twohalf")):
                for key,k in CHANNELS.items():
                    lo,hi=ds[k]
                    nlo,nhi=(nhlo,nhhi) if key=="x_HII" else (helo,hehi)
                    L,U=bounds_ratio(lo,hi,nlo,nhi)
                    ref_lo=float(row[f"{key}_{ref}_increment_lower"])
                    ref_hi=float(row[f"{key}_{ref}_increment_upper"])
                    if not (U<ref_lo or L>ref_hi):
                        overlap[name][key]+=1
                    elif len(missing)<12:
                        missing.append(dict(node=i,map=name,channel=key,
                            native=[L,U],original=[ref_lo,ref_hi]))
        if i!=N-1:raise AssertionError("OWNER_CSV_NODE_COUNT_DRIFT")
    result=dict(schema="F05_OWNER_NATIVE_INTERVAL_MPRK_STDLIB_AND_DECIMAL120_AUDIT_V1",
        original_nodes=N,maps=3,
        source_binary_sha256=sha(source),native_interval_binary_sha256=sha(native),
        source_corrector_rows_scanned=scan,nonfinite_or_inverted_intervals=nonfinite,
        native_electron_sum_cross_audit=electron_summary,
        exact_decimal_120digit_midpoint_inclusion_samples=samples,
        original_owner_increment_interval_overlap_counts=overlap,
        original_owner_increment_interval_all_overlap=all(v==N for x in overlap.values() for v in x.values()),
        first_missing=missing,
        provenance_kind="ORIGINAL_SOURCE_FINAL_CORRECTOR_ONLY",
        rust_full_26event_HHe_OTS_thermal_interval_ivp="NOT_ESTABLISHED",
        original_14_final_ledgers="OWNER_SEPARATELY_PASSED_NOT_NATIVE",
        official_tick161="NOT_ACCEPTED",canonical_delta=0,production="HOLD")
    outfile.write_text(json.dumps(result,indent=2,sort_keys=True)+"\n")
    print(json.dumps({k:result[k] for k in ("source_corrector_rows_scanned",
        "nonfinite_or_inverted_intervals",
        "original_owner_increment_interval_overlap_counts",
        "original_owner_increment_interval_all_overlap","first_missing")},sort_keys=True))
    return result

if __name__=="__main__":
    p=argparse.ArgumentParser()
    for name in ("source","native","receipt","owner_csv","owner_receipt","out"):
        p.add_argument("--"+name,type=Path,required=True)
    a=p.parse_args()
    audit(a.source,a.native,a.receipt,a.owner_csv,a.owner_receipt,a.out)
