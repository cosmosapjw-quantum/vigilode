# Hypothesis Graph

- H1 shared controller/native estimator -> primary architecture
  - H3 native RODAS lane
  - H4 BDF variable-step + step doubling
  - H5 Radau step doubling
- H2 universal step doubling -> reference/oracle lane
- H6 cost-aware controller -> requires reliable H1/H2 estimator first
- H7 joint h/inner-work adaptivity -> future SDC/ROK/BOROK/homotopy/Leja
