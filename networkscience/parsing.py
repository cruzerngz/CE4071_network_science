"""Worker funcs for transforming data files"""

FILTERED_CSV_PATH: str = "filtered.csv"
TEMPORAL_RELATIONS_PATH: str = "temporal_rels.csv"

import time
import pandas as pd
import dblp # dblp should be initialized in main already


def filter_raw_xls(xls_path: str) -> list[dblp.PersonRecord]:
    """Parses the xls file to authors that exist in the dlbp database"""

    # print("reading excel")
    raw = pd.read_excel(xls_path)
    prev = len(raw)
    raw.drop_duplicates(subset=["dblp"], inplace=True)
    print(f"Deduplicated {prev} to {len(raw)} entries")

    raw_authors = raw["name"].tolist()

    db_authors: list[dblp.PersonRecord] = []
    unmatched_authors = []

    print()
    count = 0
    for author in raw_authors:
        authors = dblp.query_person(author)
        if len(authors) == 0:
            unmatched_authors.append(author)
            continue

        count += 1
        print(f"\rfound {count}/{len(raw_authors)}", end="")
        db_authors.append(authors[0])

    # print(f"Matched {len(db_authors)} out of {len(raw_authors)} authors")
    print(f"\nUnmatched authors:")
    for unmatched in unmatched_authors:
        print(unmatched)

    db_authors_df = pd.json_normalize([dict(auth) for auth in db_authors])

    print("writing filtered data to csv: ", FILTERED_CSV_PATH)
    print()
    db_authors_df.to_csv(FILTERED_CSV_PATH)

    return db_authors

def generate_temporal_relations(authors: list[dblp.PersonRecord]) -> list[dblp.PersonTemporalRelation]:
    """Generate the temporal relations between selected authors and save the results"""

    print(f"Generating temporal relations. Estimated time: {len(authors) * 4} seconds.")
    s = time.time()
    relations = dblp.temporal_relation(authors)
    e = time.time()

    print(f"\nTime taken: {e - s} seconds")
    print(f"Saving relations to: {TEMPORAL_RELATIONS_PATH}")
    dblp.save_temporal_relation(relations, TEMPORAL_RELATIONS_PATH)

    return relations
