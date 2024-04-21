"""Data visualisations after parsing"""

import networkx as nx
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sb
import numpy as np
from scipy import stats

import typing

def graph_mapping(df: pd.DataFrame, year: int) -> dict:
    """Using the temporal relations dataframe, create the dictionary mapping
    of authors and their collaborators.

    This mapping can be passed to nx.Graph directly to construct a graph.
    """

    view = df.loc[:, ["author", str(year)]]

    mapping = {}

    for row_idx in range(len(view)):
        row = view.iloc[row_idx, :]

        # idk why nan appears
        co_auths = [c_a for c_a in str(row[str(year)]).split("::") if c_a != "nan"]

        # add to mapping if collaborators exist
        if len(co_auths) != 0:
            mapping[row["author"]] = co_auths

    return mapping


def plot_degree_distribution(graph: nx.Graph, year: int, prefix: str = None) -> typing.Tuple[int, int, float]:
    """Log-log degree dist with best fit line.

    Returns the network parameters as a tuple:
    - number of nodes
    - number of links
    - gamma as best-fit line gradient
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

    plt.close()

    return (graph.number_of_nodes, graph.number_of_edges(), -m)

def plot_gamma_progression(years: list[int], gammas: list[float], prefix: str = None):
    """Plot the progression of gamma over the years."""

    fig = plt.figure(figsize=(6, 6))
    sb.lineplot(x=years, y=gammas, color="blue", alpha=0.8, linestyle="--")

    plt.title(f"Gamma progression, {min(years)} - {max(years)}")

    if prefix is not None:
        plt.savefig(f"{prefix}_gamma_progression_{min(years)}-{max(years)}.png")
    else:
        plt.savefig(f"gamma_progression_{min(years)}-{max(years)}.png")

    plt.close()

    return


def plot_graph_prog_statistics(graphs: list[nx.Graph], years: list[int], prefix: str = None):
    """Plot the progression of graph statistics over the years."""

    stats = [
        (nx.number_of_nodes(g), nx.number_of_edges(g))
        for g in graphs
    ]

    line1_col = "gray"
    line2_col = "black"
    line1_style = "--"
    line2_style = "-"

    fig = plt.figure(figsize=(6, 6))
    # plot a line each for nodes and edges, with the left y-axis for nodes and right y-axis for edges
    ax1 = fig.add_subplot(111)
    sb.lineplot(x=years, y=[s[0] for s in stats], color=line1_col, alpha=0.8, linestyle=line1_style, ax=ax1)

    ax2 = ax1.twinx()
    sb.lineplot(x=years, y=[s[1] for s in stats], color=line2_col, alpha=0.8, linestyle=line2_style, ax=ax2)

    ax1.set_ylabel("Authors", color=line1_col)
    ax2.set_ylabel("Connections", color=line2_col)
    ax1.set_xlabel("Year")

    skip = int(len(years) / 8)
    if skip == 0:
        skip = 1

    ax1.set_xticks(years[::skip])
    ax1.set_xticklabels(years[::skip])

    plt.title(f"Collaborator connections over time ({min(years)} - {max(years)})")

    if prefix is not None:
        plt.savefig(f"{prefix}_connections_{min(years)}-{max(years)}.png")
    else:
        plt.savefig(f"connections_{min(years)}-{max(years)}.png")

    plt.close()

    return

def plot_degree_heatmap(graph: nx.Graph, year: int, prefix: str = None, filter: bool = True):
    """Plot the degree heatmap for the graph.

    The year and prefix are only used to name the output file.
    """

    x_data = []
    y_data = []

    for i, j in graph.edges():
        # if graph.degree(i) or graph.degree(j):
        #     continue

        x_data.append(graph.degree(i))
        x_data.append(graph.degree(j))

        y_data.append(graph.degree(j))
        y_data.append(graph.degree(i))

    vals = np.vstack([x_data, y_data])
    kernel = stats.gaussian_kde(vals)(vals)

    fig = plt.figure(figsize=(6, 6))

    ax = sb.scatterplot(x=x_data, y=y_data, alpha=0.25, marker="X", c=kernel, cmap="rocket_r")
    # ax = sb.heatmap([x_data,y_data], color="black", alpha=0.1)
    # ax.invert_xaxis()
    ax.invert_yaxis()

    deg_ass_corr = nx.degree_assortativity_coefficient(graph)
    plt.title(f"Degree heatmap ({deg_ass_corr:2f})")

    if prefix is not None:
        plt.savefig(f"{prefix}_degree_heatmap_{year}.png")
    else:
        plt.savefig(f"degree_heatmap_{year}.png")

    plt.close()

    return

