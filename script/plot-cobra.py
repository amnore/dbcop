#!/bin/python3
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
from pathlib import Path
from shutil import rmtree
from functools import reduce

in_dir = Path('/tmp/csv')
out_dir = Path('/tmp/plot')

if out_dir.exists():
    rmtree(out_dir)
out_dir.mkdir()

for in_file in in_dir.glob('*.csv'):
    data = pd.read_csv(in_file)
    verifiers = [
        'si',
        'oopsla',
        'cobra',
        'cobra(si)',
        'cobra(nogpu)'
    ]

    if all(map(lambda s: s.startswith('chengRW-'), data['param'])):
        # plot chengRW
        for i in range(len(data['param'])):
            data['param'][i] = int(data['param'][i][len('chengRW-'):])
        data.sort_values('param', inplace=True)
        plot = data.plot('param', verifiers)
        plot.set_title('Cobra one-shot-chengRW')
        plot.set_ylim([0, 100])
    else:
        # plot different datasets
        for i in range(len(data['param'])):
            data['param'][i] = data['param'][i][:-len('-10000')]
        plot = data.plot('param', verifiers, 'bar')
        plot.set_title('Cobra one-shot-10k')
        plot.set_ylim([0, 60])
        plot.set_xticks(plot.get_xticks(), plot.get_xticklabels(), rotation=0)

    plot.set_ylabel('Time (s)')
    plt.savefig(out_dir / in_file.name.replace('.csv', '.png'))
