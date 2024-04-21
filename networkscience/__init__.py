# hellow
"""Main entry point for this package."""

from networkscience import args
from networkscience import parsing
from networkscience import visuals

import networkx as nx
import pandas as pd
import numpy as np
import argparse
import dblp

# cli arguments: xml file, sqlite file, xls file, relations file
# xml file: contains the data
# sqlite file: contains the data
# xls file: contains the data
# relations file: contains the data
parser = argparse.ArgumentParser("CE4071 DBLP collaboration network program")
# parser.description = """
# This program operates in stages. If a stage has been executed and data saved,
# the program can resume execution from that saved stage.\n\n\n\n\n\n\n\n

# The stages are as follows:\n\n

# - xml to sqlite parsing         (required arg: path to dblp.xml)\n
# - xls to filtered csv           (required arg: path to dblp.sqlite)\n
# - filtered csv to relations     (required arg: path to dblp.xls)\n
# - graph construction            (required arg: path to relations.csv)\n
# """

parser.add_argument("--year-start", help="start year for the temporal relations", required=True)
parser.add_argument("--year-end", help="end year for the temporal relations", required=True)

parser.add_argument("--file-prefix", help="additional prefix for output files", metavar="PFX", default=None)

parser.add_argument("--xml", help="dblp xml file path")
parser.add_argument("--sqlite", help="sqlite file path")
parser.add_argument("--xls", help="xls input file")
parser.add_argument("--csv", help="parsed xls file path")
parser.add_argument("--relations", help="relations file containing the data")

def main():
    args = parser.parse_args()

    # database init
    match (args.xml, args.sqlite):
        case (None, None):
            print("No database specified. Defaulting to dblp.sqlite")
            dblp.init_from_sqlite()

        case (xml, None):
            dblp.init_from_xml(xml)

        case (_, sqlite):
            dblp.init_from_xml(sqlite)

    # can be initialized in various ways
    rel_csv: pd.DataFrame

    # data parsing from checkpoints
    match (args.xls, args.csv, args.relations):
        case (None, None, None):
            print("An xls, filtered csv or parsed relations file must be provided")
            exit(1)

        case (xls, None, None):
            # print("Parsing xls file")
            authors = parsing.filter_raw_xls(xls)
            relations = parsing.generate_temporal_relations(
                authors,
                args.year_start,
                args.year_end
            )

        case (_, csv, None):
            df_csv = pd.read_csv(csv)
            df_csv.fillna("", inplace=True)
            authors_d = df_csv.to_dict(orient="records")

            authors = dblp.PersonRecord.from_dicts(authors_d)
            relations = parsing.generate_temporal_relations(
                authors,
                args.year_start,
                args.year_end
            )

            if args.file_prefix is not None:
                rel_csv = pd.read_csv(f"{args.file_prefix}_{parsing.TEMPORAL_RELATIONS_PATH}")
            else:
                rel_csv = pd.read_csv(parsing.TEMPORAL_RELATIONS_PATH)

        case (_, _, rel):
            rel_csv = pd.read_csv(rel)
            pass

        case _:
            print("unmatched case, exiting")
            exit(1)

    # process temporal relations from here
    gammas = []
    for year in range(int(args.year_start), int(args.year_end) + 1):
        mapping = visuals.graph_mapping(rel_csv, year)
        # mappings.append(mapping)
        gamma = visuals.plot_degree_distribution(nx.Graph(mapping), year, args.file_prefix)
        gammas.append(gamma)

    print("Gammas for each year:", gammas)

if __name__ == "__main__":
    main()
