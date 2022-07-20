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

    param_names = ['n_sessions', 'n_txns', 'n_ops', 'n_vars']
    params = np.transpose([[int(x) for x in y.split('_')] for y in data['param']])
    for i in range(4):
        if params[i][0] != params[i][1]:
            changed_param_name = param_names[i]
            data[changed_param_name] = params[i]

    for i in range(len(data['oopsla'])):
        if data['oopsla'][i] == 60:
            data['oopsla'][i] = 180

    plot = data.plot(changed_param_name, [
        'si',
        'oopsla',
        'cobra',
        'cobra(si)',
        'cobra(nogpu)'
    ])
    plot.set_ylabel('Time (s)')
    plot.set_title(in_file.name.replace('.csv', '') + ' ' +
                   reduce(lambda x,y: x+','+y, ['{}={}'.format(param_names[i], params[i][0]) for i in range(4) if param_names[i]!=changed_param_name]))
    plot.set_ylim([0, 20])
    plt.savefig(out_dir / in_file.name.replace('.csv', '.png'))
