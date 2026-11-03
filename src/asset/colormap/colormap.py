import numpy as np
import matplotlib.pyplot as plt

cmap_name = 'Greys';
cmap = plt.get_cmap(cmap_name)
values = cmap(np.linspace(0, 1, 256))
values_u8 = (values * 255).astype(np.uint8)

values_u8.tofile(f'{cmap_name}.bin')
