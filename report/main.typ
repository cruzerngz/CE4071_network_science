#set text(font: "Nimbus Roman")
#set heading(numbering: "1.1.")
#set par(justify: true)

// table style - I want to emulate the look seen in papers
#set table(
  stroke: (x, y) => (
    y: if y <= 1 {1pt} else {0.1pt},
    left: 0pt,
    right: 0pt,
    bottom: 1pt,
  ),
)

// #show raw.where(lang: l => l = none): code => {
//   block(
//     outset: 5pt,
//     breakable: false,
//     width: 100%
//   )[#text(size: 0.9em)[#code]]
// }

// cover page
#align(center, text(size: 1.5em, weight: "bold")[

  #image("media/ntu_logo.svg", width: 75%)

  CE4071 Network Science
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  2023/2024 Semester 2 course project:

  _Network Science based analysis of collaboration network of Data Scientists_
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  #linebreak()
  Ng Jia Rui: U2020777D
  #linebreak()
  #linebreak()
  #linebreak()
  SCHOOL OF COMPUTER SCIENCE AND ENGINEERING
  NANYANG TECHNOLOGICAL UNIVERSITY
])
#pagebreak()

#set page(numbering: "1")
#outline(indent: true)
#linebreak()
#outline(title: "Tables", target: figure.where(kind: table))
#linebreak()
#outline(title: "Figures", target: figure.where(kind: image))

#pagebreak()
= Overview

= Dataset
As this project requires parsing of some input data and forming connections through records in the DBLP dataset, performing queries on the dataset over the network is unfeasible, due to rate limits imposed by the DBLP API @dblp-rate-limits.
As such, the dataset has to be downloaded locally for processing.

// sql vs pandas
== Data representation
DBLP provides an up-to-date XML dataset of all publications and authors @dblp-dataset-online.
However, the data is not queryable in this format, and it has to be transformed.

In this project, I have decided to parse the XML data and transform it into an SQLite database, which enables quick queries to be made when constructing the collaboration network.

== Parsing and Querying
The XML dataset is `3GB` in size when unzipped. When parsed and deserialized into it's in-memory representation in Rust, memory usage can peak at over `15GB`.

To reduce memory usage, the dataset has to be parsed in chunks of `1000` XML top-level elements at a time. This process takes about 3 minutes to complete.

Indexes to the following columns are created:
- Author name
- Author home page URL, unique
- Year of publication
- Author(s) of publication
- Citation(s) of publication

After all filtration steps, the dataset is reduced to approximately `1.7GB` in size.
Additional modifications to the dataset are described in the table below.

#figure(
  caption: [Dataset modifications],
  table(
    align: left,
    columns: (auto, auto),
    [*modification*], [*description*],
    [citation filtering],[
      Each paper may have none, one, or multiple citations. Some citations have been added to the dataset as blank strings (`""`), or sequences of ellipses (`...`).

      Citations containing blank strings or strings that contain no alphabetic characters are removed.
    ],
    [vector squashing],[
      For columns that contain vectors, or sequences of values, these values are squashed into a single string, separated by a delimiter.
      This is performed as SQLite does not have a vector type.

      To aid in exact author querying for publications,
      the same delimiter is also added to the start and end of a string.

      ```rs
      // before
      let before = ["value_1", "value_2", "value_3"];
      // after
      let after = "::value_1::value_2::value_3::";
      ```

      A sample query string to query all publications from a specific author can then be:
      ```sql SELECT * FROM publications WHERE authors LIKE "%::author_name::%"```
    ],
    [discard data],[
      Data that is not used for this project is not included in the final dataset.
      This includes, but is not limited to:
      - Web pages that are not author home pages
      - editors
      - addresses
      - volume number, pages, and other publication metadata
      - custom notes ```xml <note/>```
      - related works ```xml <rel/>```
    ],
  )
)

== Query Library
A simple query library, built on top of SQLite, is created to address the needs of this project.
This library provides query abstractions for:
- Author search with alias matching
- Author-publication searches
- Author-collaborator matching with yearly granularity

This library also provides objects (`DblpRecord`, `PersonRecord`) for each table in the dataset so that queries and their results can be retrieved in a type-safe manner.

The dataset can be queried directly using SQL, if the query library does not provide the necessary functionality.

= Input data

== Input filtering and Association
The main program (`project.py`) will operate on some input data, which is a dirty subset of the DBLP dataset.
Inside this subset, the following operations are performed to clean the data:

- Deduplication of entries
- Author name lookup and matching to dataset
// - Temporal relation generation of author-collaborators

Entries are deduplicated by removing matching DBLP homepage paths.

As stated in the assignment, names in the input file do not directly match with names in the dataset.
To address this, a 2-stage search is used to find the closest matching author name in the dataset.

This search takes advantage of the following:
- Input name sections are ordered the same way as names in the dataset
- Input names may have certain sections missing in between name segments, but never at start or end (e.g. middle name/other initials)
- Colliding names in the dataset are assigned a 0-padded 4-digit monotonically increasing suffix (e.g. John Doe 0001)
- Authors can have aliases, which are also searched for in the dataset when the first search fails

Using the sample input file as a reference, this search method successfully matches $95%$ of input names to names in the dataset. (1024 of 1079 entries, after deduplication from 1220 entries)

#pagebreak()
#bibliography("bib.yaml")

