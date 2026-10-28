ClearAll[x,d,y0,q1,q2,q3,q4];
p[x_]:=y0+q1*x+q2*x^2+q3*x^3+q4*x^4;
right=p[1]-d*(q1+2*q2+3*q3+4*q4)+d^2*(q2+3*q3+6*q4+d*(-q3-4*q4+d*q4));
left=y0+x*q1+x^2*(q2+x*(q3+x*q4));
ExportString[<|"right_anchor_residual"->Expand[right-p[1-d]],"left_anchor_residual"->Expand[left-p[x]],"scope"->"Polynomial rearrangement only; does not certify floating point rounding or overflow absence."|>,"RawJSON"]
