# G4-S4 Actual Prefix Kernel Gate

이 node는 G4-S3의 retrospective prefix-cost model을 실제 Rust Arnoldi/GMRES prefix로 교체한다.

- E-K: fused-phi augmented Arnoldi prefix를 만든 뒤 같은 basis/Hessenberg를 계속 확장한다.
- R-JF: restart-free GMRES prefix를 만든 뒤 같은 basis/least-squares state를 계속 확장한다.
- prefix 1은 negative control, prefix 2는 n=32 calibration에서 선택했다.
- canonical holdout은 n=128,512에서 threshold를 변경하지 않고 평가한다.
- active switching은 이 node에서 금지한다.
