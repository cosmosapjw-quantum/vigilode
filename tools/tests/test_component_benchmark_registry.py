import unittest
import json
from pathlib import Path
from component_benchmark.registry import CRATES, ADDITIONAL_VARIANTS, inventory

class RegistryTest(unittest.TestCase):
    def test_all_five_crates_including_specialized_modules(self):
        rows=inventory(Path(__file__).resolve().parents[2])
        self.assertEqual({r["crate"] for r in rows},set(CRATES))
        self.assertTrue(any("polynomial_action" in r["source_path"] for r in rows))
        self.assertTrue(all(r["evidence_status"]=="HOLD" for r in rows))
        variants={r["component_id"].split("#")[-1] for r in rows if r["kind"]=="algorithm_variant"}
        self.assertTrue({"BDF1","BDF2","RadauIIA1","RadauIIA3","EXPRB2","EXPRB43","PEXPRB54S4","Integral","PI","Predictive","LegacyEstimateOnly","CertifiedLeja","StageH5","WindowHomotopy"} <= variants)
        self.assertLess(len(rows),300)

    def test_static_variant_registry_matches_coverage_map(self):
        root=Path(__file__).resolve().parents[2]
        rows=inventory(root)
        ids=[r['component_id'] for r in rows]
        variants=[r for r in rows if r['kind']=='algorithm_variant']
        coverage=json.loads((root/'research/component_benchmark_20261018/COMPONENT_COVERAGE_MAP.json').read_text())
        self.assertEqual(len(rows),246)
        self.assertEqual(len(variants),94)
        self.assertEqual(len(ids),len(set(ids)))
        self.assertEqual(set(ids),{r['component_id'] for r in coverage['current_ids']})
        self.assertEqual(len(coverage['integrated_variants']),73)
        self.assertEqual(coverage['proposed_variants'],[])
        self.assertEqual(sum(map(len,ADDITIONAL_VARIANTS.values())),73)
        self.assertEqual({r['component_id'] for r in coverage['integrated_variants']},
                         {module+'#'+variant for module, entries in ADDITIONAL_VARIANTS.items()
                          for variant, anchor in entries})
        self.assertTrue(all(r['recommended_action']=='need to optimize' and r['evidence_status']=='HOLD'
                            for r in rows))

    def test_shared_module_variants_remain_distinct(self):
        ids={r['component_id'] for r in inventory(Path(__file__).resolve().parents[2])}
        expected={
            'rodas5p-integrators:src::rodas5p_fast#DenseLegacyLU',
            'rodas5p-integrators:src::rodas5p_fast#DenseColumnExtentsLU',
            'rodas5p-integrators:src::rodas5p_fast#BandedIndexed',
            'rodas5p-integrators:src::rodas5p_fast#BandedSlices',
            'rodas5p-integrators:src::rodas5p_matrix_free_fast#MatrixFreeGMRESInto',
            'rodas5p-integrators:src::rodas5p_matrix_free_fast#MatrixFreeLSWorkspace',
            'rodas5p-krylov:src::gmres#GMRESIncremental',
            'rodas5p-krylov:src::gcrodr#GCRODRPolicy',
            'rodas5p-core:src::polynomial_action#JointPhiChebyshev',
            'rodas5p-core:src::polynomial_action#JointPhiLaguerre',
            'rodas5p-integrators:src::output#ProductionIndexedClock',
            'rodas5p-integrators:src::output#ResearchReplayClock',
            'rodas5p-integrators:src::collective_direct#RankTwoWoodbury',
        }
        self.assertTrue(expected <= ids)
