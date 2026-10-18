"""Mock-only hard-cap controls for the one-shot P3 driver."""
from __future__ import annotations

import importlib.util
import json
import platform
import subprocess
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

DRIVER=Path(__file__).with_name('run_ncp_p3.py')
spec=importlib.util.spec_from_file_location('vigilode_p3_driver',DRIVER)
p3=importlib.util.module_from_spec(spec)
spec.loader.exec_module(p3)


class DriverHardCapTests(unittest.TestCase):
    def test_near_expired_p2_refuses_wait_and_transition(self):
        with tempfile.TemporaryDirectory() as directory:
            base=Path(directory)
            marker=base/'P2_QUEUES_FINISHED.json'
            now=time.time()
            ledger=base/'NCP_LEASE_LEDGER.json'
            ledger.write_text(json.dumps({'schema':'host-lease-v1','leases':[{
                'id':'frozen-p2','phase':'P2','host':platform.node(),'start':now-30,
                'deadline':now+10,'start_monotonic':time.monotonic()-30,
                'reserved_seconds':40,'persistent':True}]}))
            with self.assertRaisesRegex(RuntimeError,'P2_LEASE_CLEANUP_RESERVE_REACHED'):
                p3.active_p2(ledger,{'lease_id':'frozen-p2'},platform.node())
            with self.assertRaisesRegex(RuntimeError,'P2_LEASE_CLEANUP_RESERVE_REACHED'):
                p3.wait_p2(marker,3600,now+10)
            self.assertFalse(marker.exists())

    def test_stalled_child_kills_only_owned_group_and_keeps_raw_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            logs=Path(directory)/'commands';logs.mkdir()
            with self.assertRaisesRegex(RuntimeError,'fake_HOST_LEASE_TIMEOUT'):
                p3.run_logged_command(logs,'fake',[sys.executable,'-c','import time;time.sleep(5)'],
                    Path(directory),{},time.monotonic()+0.12,'source','config','driver')
            row=json.loads((logs/'fake.json').read_text())
            self.assertIs(row['timed_out'],True)
            self.assertEqual(row['raw_status'],'ENVIRONMENT_FAILURE:HOST_LEASE_TIMEOUT')
            self.assertTrue((logs/'fake.stdout').is_file())
            self.assertTrue((logs/'fake.stderr').is_file())
            with patch.object(p3.subprocess,'Popen',side_effect=AssertionError('launched')):
                with self.assertRaisesRegex(RuntimeError,'P3_LEASE_CLEANUP_RESERVE_REACHED'):
                    p3.run_logged_command(logs,'expired',['/bin/true'],Path(directory),{},
                                          time.monotonic()-1,'source','config','driver')

    def test_nested_session_cleanup_preserves_unrelated_control(self):
        with tempfile.TemporaryDirectory() as directory:
            base=Path(directory);logs=base/'commands';logs.mkdir()
            nested_pid=base/'nested.pid'
            controller=base/'controller.py'
            controller.write_text(
                'import pathlib,subprocess,sys,time\n'
                'child=subprocess.Popen([sys.executable,"-c","import time;time.sleep(10)"],start_new_session=True)\n'
                'pathlib.Path(sys.argv[1]).write_text(str(child.pid))\n'
                'time.sleep(10)\n')
            unrelated=subprocess.Popen([sys.executable,'-c','import time;time.sleep(10)'],
                                       start_new_session=True)
            try:
                with self.assertRaisesRegex(RuntimeError,'nested_HOST_LEASE_TIMEOUT'):
                    p3.run_logged_command(logs,'nested',[sys.executable,str(controller),str(nested_pid)],
                        base,{},time.monotonic()+0.5,'source','config','driver')
                self.assertTrue(nested_pid.is_file())
                child_pid=int(nested_pid.read_text())
                cleanup=json.loads((logs/'nested.cleanup.json').read_text())
                self.assertIn(child_pid,[member['pid'] for group in cleanup['verified_groups']
                                         for member in group['members']])
                signals=cleanup['signals']
                self.assertEqual(signals[0]['signal'],'SIGSTOP')
                nested_kill=next(i for i,row in enumerate(signals)
                                 if row['target']=='verified_descendant_group')
                controller_kill=next(i for i,row in enumerate(signals)
                                     if row['target']=='controller' and row['signal']=='SIGKILL')
                self.assertLess(nested_kill,controller_kill)
                self.assertEqual(cleanup['controller_reap'],'REAPED')
                self.assertFalse(p3.process_identity(child_pid) and
                                 p3.process_identity(child_pid)['state']!='Z')
                self.assertIsNone(unrelated.poll())
                self.assertNotIn(unrelated.pid,[member['pid'] for group in cleanup['verified_groups']
                                               for member in group['members']])
            finally:
                unrelated.terminate()
                unrelated.wait(timeout=2)


if __name__=='__main__':
    unittest.main()
