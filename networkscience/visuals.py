"""Data visualisations after parsing"""

import networkx as nx
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sb
import numpy as np


def graph_mapping(df: pd.DataFrame, year: int) -> dict:
    """Using the temporal relations dataframe, create the dictionary mapping
    of authors and their collaborators.

    This mapping can be passed to nx.Graph directly to construct a graph.
    """

    view = df.loc[:, ["author", str(year)]]
    # print(view.head())

    mapping = {}

    for row_idx in range(len(view)):
        row = view.iloc[row_idx, :]

        co_auths = [c_a for c_a in str(row[str(year)]).split("::") if c_a]
        mapping[row["author"]] = co_auths

    return mapping


def plot_degree_distribution(graph: nx.Graph, year: int, prefix: str = None) -> int:
    """Log-log degree dist with best fit line.

    Returns the gamma for the best fit line.
    """

    data = [list(t) for t in zip(*nx.degree(graph))]
    _df = pd.DataFrame({
        "author": data[0],
        "degree": data[1]
    })

    dist = _df["degree"].value_counts().sort_values()

    fig = plt.figure(figsize=(6, 6))

    sb.scatterplot(x=dist.index, y=dist, size=10, color="black", legend=False)

    # plot a best-fit line of this data using nump
    x = [x for x in dist.index]
    y = [y for y in dist]
    m, b = np.polyfit(np.log(x), np.log(y), 1)

    sb.lineplot(x=x, y=np.exp(m*np.log(x) + b), color="blue", alpha=0.5, linestyle="--")
    # label the line with gradient
    plt.text(0.5, 0.5, f"gamma = {-m:.2f}", color="blue", transform=plt.gca().transAxes)

    # x_end = max(x[:len(x) - 2]) / max(x)
    # y_min = min(y) / max(y)
    # plt.annotate("asdasd", xy=(x_end * 0.8, y_min * 0.8), xycoords='axes fraction', color="blue")

    plt.xscale('log')
    plt.yscale('log')
    plt.title(f"Degree distribution for {year}")

    if prefix is not None:
        plt.savefig(f"{prefix}_degree_dist_{year}.png")
    else:
        plt.savefig(f"degree_dist_{year}.png")

    return -m
