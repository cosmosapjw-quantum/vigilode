cd /root/vigilode-ncpf-20261021T1031Z-5fd6e45
echo "B start $(date -u +%FT%TZ)" >> logs/PHASES.log
c=40
for arm in VODE_MF20 VODE_MF21 VODE_MF22 JULIA_RODAS5P DIFFSOL_BDF; do
  taskset -c $c python3 harness/run_comparators.py /root/vigilode-ncpf-20261021T1031Z-5fd6e45 $arm $c 2 7 > logs/B_$arm.out 2> logs/B_$arm.err &
  c=$((c+1))
done
wait
echo "B end $(date -u +%FT%TZ)" >> logs/PHASES.log
touch logs/PHASE_B.DONE
