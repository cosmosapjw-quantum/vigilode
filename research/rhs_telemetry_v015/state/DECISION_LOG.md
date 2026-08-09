# Decision log

1. Keep recommendations advisory and dispatcher inactive.
2. Use directional, not magnitude-dominated, analysis for recommendation.
3. Force high-risk rows to independent Rayon GMRES.
4. Require transactional shadow testing before activation.
5. Defer BGMRES-DR until actual dispatched traces justify it.
6. Freeze BDF/Radau optimization as comparator work.
