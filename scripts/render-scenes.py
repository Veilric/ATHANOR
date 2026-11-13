#!/usr/bin/env python3
"""Render the actual Rune UI; fixture data is confined to explicit scene mode."""
from pathlib import Path
import os
import subprocess
import sys
root=Path(__file__).resolve().parent.parent
binary=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else root/'target/release/athanor'
env=os.environ.copy()
env.setdefault('XDG_CACHE_HOME',str(root/'.local-run/cache'))
out=root/'artifacts/scenes';out.mkdir(parents=True,exist_ok=True)
for scene in ['calculator','units','calendar','currency-offline','search','error','settings','pinned-idle']:
    size='330x53' if scene=='pinned-idle' else '330x550' if scene=='settings' else '330x249' if scene=='search' else '330x238'
    for scale in ['1','2']:
        stem=out/f'{scene}-{scale}x'
        subprocess.run([str(binary),'--window','athanor','--scene',scene,'--size',size,'--scale',scale,
                        '--snap',str(stem.with_suffix('.png')),'--tree',str(stem.with_suffix('.json'))],env=env,cwd=root,check=True)
        print(stem.name,flush=True)
