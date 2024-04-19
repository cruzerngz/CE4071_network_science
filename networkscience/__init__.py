# hellow
"""Main entry point for this package."""

from networkscience import args
from networkscience import parsing

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

    # data parsing from checkpoints
    match (args.xls, args.csv, args.relations):
        case (None, None, None):
            print("An xls, filtered csv or parsed relations file must be provided")
            exit(1)

        case (xls, None, None):
            # print("Parsing xls file")
            authors = parsing.filter_raw_xls(xls)
            relations = parsing.generate_temporal_relations(authors)


        case (_, csv, None):
            df_csv = pd.read_csv(csv)
            df_csv.fillna("", inplace=True)
            authors_d = df_csv.to_dict(orient="records")

            authors = dblp.PersonRecord.from_dicts(authors_d)
            relations = parsing.generate_temporal_relations(authors)


        case (_, _, rel):
            print("rel matched")
            pass

        case _:
            print("unmatched case")


    # print("Hello World")

if __name__ == "__main__":
    main()
