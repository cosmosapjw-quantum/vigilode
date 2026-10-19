import json,tempfile,unittest
from pathlib import Path
import numpy as np
from campaign import dump
class SerializationTests(unittest.TestCase):
    def test_numpy_counters_are_lossless_json(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'a.json';dump(p,{'nfev':np.int32(17),'njev':np.int64(2),'flag':np.bool_(True),'value':np.float64(.5)})
            self.assertEqual(json.loads(p.read_text()),{'nfev':17,'njev':2,'flag':True,'value':.5})
    def test_nonfinite_not_encoded(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(ValueError):dump(Path(d)/'a.json',{'v':np.float64(np.nan)})
    def test_unknown_object_not_stringified(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(TypeError):dump(Path(d)/'a.json',{'v':object()})
if __name__=='__main__':unittest.main(verbosity=2)
