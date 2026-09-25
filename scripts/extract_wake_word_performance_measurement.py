#!/usr/bin/env python3
import argparse, json, os, statistics
from pathlib import Path

p=argparse.ArgumentParser()
p.add_argument('--acceptance-report', required=True)
p.add_argument('--output', required=True)
p.add_argument('--platform', required=True)
p.add_argument('--commit-sha', required=True)
p.add_argument('--runner', required=True)
a=p.parse_args()
r=json.loads(Path(a.acceptance_report).read_text())
fixtures=r.get('fixtures', [])
lat=[x['inference_wall_time_ms'] for x in fixtures]
audio=sum(x.get('audio_ms',0) for x in fixtures)
wall=sum(lat)
cpu=r.get('process_cpu_time_ms',0)
# This is corpus-active CPU utilization, not idle CPU; keep the name explicit.
active_cpu=(100.0*cpu/wall) if wall else 0.0
metrics={
 'corpus_active_cpu_percent':round(active_cpu,3),
 'idle_cpu_percent':round(float(r.get('idle_cpu_percent',0)),3),
 'idle_observation_ms':int(r.get('idle_observation_ms',0)),
 'runtime_memory_mib':round((r.get('peak_resident_memory_bytes') or 0)/1048576,3),
 'inference_latency_ms':round(statistics.mean(lat),3) if lat else 0.0,
 'inference_p95_ms':sorted(lat)[max(0,int(len(lat)*0.95)-1)] if lat else 0,
 'corpus_audio_ms':audio,'corpus_inference_wall_ms':wall,
 'max_real_time_factor':max((x.get('real_time_factor',0) for x in fixtures),default=0),
 'wake_to_command_asr_ms':int(r.get('wake_to_command_asr_ms',0)),
 'command_start_ms':int(r.get('command_start_ms',0)),
 'total_activation_ms':int(r.get('total_activation_ms',0)),
 'pre_roll_startup_ms':int(r.get('pre_roll_startup_ms',0)),
 'pre_roll_samples':int(r.get('pre_roll_samples',0)),
 'inference_threads':r.get('inference_threads')
}
pending=['continuous_asr_idle_cpu_percent','repeated_cycle_resource_delta']
if r.get('continuous_asr_idle_cpu_percent') is not None:
 metrics.update({
  'continuous_asr_idle_cpu_percent':round(float(r.get('continuous_asr_idle_cpu_percent')),3),
  'continuous_asr_observation_ms':int(r.get('continuous_asr_observation_ms',0)),
  'continuous_asr_audio_ms':int(r.get('continuous_asr_audio_ms',0)),
  'continuous_asr_processed_audio_ms':int(r.get('continuous_asr_processed_audio_ms',0))
 })
 pending=['repeated_cycle_resource_delta']
out={
 'schema_version':1,'platform':a.platform,'commit_sha':a.commit_sha,'runner':a.runner,
 'measured_at':os.environ.get('MEASURED_AT','github-actions'),
 'metrics':metrics,
 'pending_metrics':pending
}
Path(a.output).write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
