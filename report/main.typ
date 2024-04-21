#set text(font: "Nimbus Roman")
#set heading(numbering: "1.1.")
#set par(justify: true)

#import "@preview/algo:0.3.3": algo, i, d, comment, code

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

#let h_link(target, label) = {
  link(target)[#(underline(text(label, fill: blue), stroke: blue))]
}

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
In hindsight, I have spent much more time on transforming the dataset and constructing the author collaboration network than I have on the analysis of the network.
On the bright side, this dataset can be used for future analysis, and the network can be queried for various metrics.

Feel free to reuse or modify the code for your own analysis.

= Dataset
As this project requires parsing of some input data and forming connections through records in the DBLP dataset, performing queries on the dataset over the network is unfeasible, due to rate limits imposed by the DBLP API @dblp-rate-limits.
As such, the dataset has to be downloaded locally for processing.

// sql vs pandas
== Data representation
DBLP provides an up-to-date XML dataset of all publications and authors @dblp-dataset-online.
However, the data is not queryable in this format, and it has to be transformed.

In this project, I have decided to parse the XML data and transform it into an SQLite database, which enables quick queries to be made when constructing the collaboration network.

== Parsing and Querying <parse_query>
The XML dataset #cite(label("10.14778/1687553.1687577")) is `3GB` in size when unzipped. When parsed and deserialized into it's in-memory representation in Rust, memory usage can peak at over `15GB`.

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
      - Web pages that are not author home pages @dblp-homepage-record
      - editors
      - addresses
      - volume number, pages, and other publication metadata
      - custom notes ```xml <note/>```
      - related works ```xml <rel/>```
    ],
  )
)

== Query Library
A query library (`dblp`) built on top of SQLite, is created to address the needs of this project.
This library provides query abstractions for:
- Author search with alias matching
- Author-publication searches
- Author-collaborator matching with yearly granularity

This library also provides objects (`DblpRecord`, `PersonRecord`) for each table in the dataset so that queries and their results can be retrieved in a type-safe manner.

The dataset can be queried directly using SQL, if the query library does not provide the necessary functionality.

= Input data

== Input filtering and Association <input_filt>
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
- Authors aliases are also searched for in the dataset when the first search fails

Using the sample input file as a reference, this search method successfully matches $95%$ of input names to names in the dataset. (1024 of 1079 entries, after deduplication from 1220 entries)

== Staged execution
The program performs the following operations in sequence, with each stage building on the previous. Execution can resume from any stage, as long as the required files are present.

#figure(
  caption: [Staged execution of the program and their outputs],
  table(
    align: left,
    columns: (auto, auto, auto, auto),
    [*stage*], [*description*], [*required files*], [*output*],
    [1], [Parse XML dataset], [none], [sqlite database file],
    [2], [Filter input data and associate authors with dataset], [sqlite database, input xls file], [filtered authors file],
    [3], [Construct temporal relations between authors and their collaborators], [sqlite database, filtered authors file], [temporal relations file],
    [4], [Construct author collaboration network and visualizations], [temporal relations file], [visualizations],
  )
) <staged_execution>

= Output data
The program outputs files as described in @staged_execution.
The final output before data visualization is the temporal relations of every author against every other author they have collaborated with over time.

This data is represented as a csv file, with the first column containing authors, and every subsequent column containing the range of years that are specified when the program is run. By default, this range is from 1961 to the current year.

Each cell contains the collaborators for a particular author up to and including the year specified in the column header.
This data does not take into account authors that publish works without any collaborators (@author_network_limitation).

= Analysis
For the sample input file, all analysis is performed within the date range of 2000 to 2024.

The author collaboration network appears to grow linearly in size with the number of authors, and the number of edges grows quadratically with the number of authors.
Almost all authors are connected in one giant component, with only a few disconnected components.

#figure(
  caption: [Growth of author collaboration network from 2000 to 2024],
  image(
    "./media/report_connections_2000-2024.png",
    width: 75%
  )
)

The network closely resembles a scale-free network, with a few authors having a large number of collaborators, and most authors having a small number of collaborators.
In 2024, the power law exponent $gamma = 1.3$.

#figure(
  caption: [Power law fit of the author collaboration network in 2000 and 2024],
  stack(
    dir: ltr,
    image(
      "./media/report_degree_dist_2000.png",
      width: 55%
    ),
    image(
      "./media/report_degree_dist_2024.png",
      width: 55%
    )
  )
)

On inspection of the network for 2024, the network is determined to be slightly assortative, with a value of $0.2$ in 2024. The degree correlation over time increases slightly.
This suggests that over time, new papers are more likely to be published by authors with other authors that have a similarly large number of collaborators, than authors with a small number of collaborators.

#figure(
  caption: [Scatterplot-heatmap of degree correlation for 2012 and 2024],
  stack(
    dir: ltr,
    image(
      "./media/report_degree_heatmap_2012.png",
      width: 55%
    ),
    image(
      "./media/report_degree_heatmap_2024.png",
      width: 55%
    )
  )
)

== Comparison with a random network
When compared to a random network with the same number of nodes, edges and average degree, the author collaboration network will have a higher clustering coefficient and a lower diameter, assuming that each link between authors has the same path length.

The probability of selecting an author with a very high degree is also higher in the author collaboration network. The degree distribution of a random network will more closely resemble a binomial distribution (when $10^2 <= N <= 10^3$, $N approx.eq 700$), while the author collaboration network will resemble a power law distribution.

= Transformation
The next section details pseudocode for transforming the given author network to one that contains a smaller giant component and a larger number of disconnected components, along with a configurable maximum number of collaborators per author.
The diversity of authors is also be equal to or greater than the original network.

The dataset does not contain accurate information that can enable the construction of such a network, so no implementation will be provided.

#figure(
  caption: [Additional author fields required to transform the author-collaborator network],
  table(
    columns: (auto, auto),
    align: left,
    [*field*], [*note*],
    [country],[Not explicitly given for each author in the dataset],
    [institution], [Available for some authors only],
    [expertise], [Randomly assigned to authors from the input file]
  )
)

An algorithm for transforming the author collaboration network is outlined below.

During each iteration of the algorithm, a random edge is selected from the network, and the importance of the authors on the edge is calculated.

As high-degree nodes are more likely to be selected, the giant component will shrink over time, and the number of disconnected components will increase.

the importance of an author is calculated as the composite of it's degree, the inverse percentages of authors from the same country, institution, and expertise; with respect to the total number of authors in the network.
The importance of each author on the selected edge is compared, and the author with the lower importance has a random edge disconnected.

The algorithm terminates once the network's maximum degree is less than or equal to the specified target.

#figure(
  caption: [Algorithm for iteratively transforming the author collaboration network],
  algo(
  title: "TransformNetwork",
  parameters: ("CollaborationNetwork","KMax",),
  block-align: horizon,
  radius: 2pt,
)[
  #comment([iterate until K_max is satisfied])
  while MAX(CollaborationNetwork.degrees()) > KMax:#i\

    let edge $<-$ CollaborationNetwork.edges().random()\
    let author1, author2 $<-$ edge.endpoints()\

    #comment([author importance calculation])
    let importance1 $<-$ author1.importance()\
    let importance2 $<-$ author2.importance()\

    if importance1 > importance2:#i\
      author2.remove_edge(author2.edges().random())#d\
    else:#i\
      author1.remove_edge(author1.edges().random())#d\

  #d\
])

#figure(
  caption: [Algorithm for determining author importance],
  algo(
  title: "AuthorImportance",
  parameters: ("Author",),
  block-align: horizon,
  radius: 2pt,
)[
  let importance $<-$ 0.0\
  importance += Log10(Author.degrees().size())\

  #comment([authors that are less represented are more important])
  let imp_diversity $<-$ Author.graph().size() / Author.graph().count(Author.country)\

  let imp_institution $<-$ Author.graph().size() / Author.graph().count(Author.institution)\

  let imp_expertise $<-$ Author.graph().size() / Author.graph().count(Author.expertise)\

  importance += imp_diversity + imp_institution + imp_expertise\

  return importance\
])

= Limitations

== Author matching
During the author association phase (@input_filt), `dblp` will not be able to match author names with non-ascii characters, as characters with ligatures have been transformed to the nearest ascii equivalents.

In the raw XML dataset, these characters are represented as XML references taken from the #h_link("https://dblp.uni-trier.de/xml/dblp.dtd", "dblp data type definition"), and as such are not supported by the xml parser.

```rs
/// Matcher for XML references
/// They follow this format:
/// &xxxxx;
const XML_REF_REGEX: &str = "&[[:alpha:]]+;";
```

For example, the name "zsolt istvan" (Zsolt István) exists in the DBLP dataset as "`zsolt istv&aacute;n`", so it will not be matched by this search algorithm.

As the fraction of authors containing references in the dataset  is small, the impact of this limitation is minimal.

== Author relation construction
Due to the large size of the dataset, the temporal relations construction phase takes a substantial amount of time (~4s per author).
Using the sample input file as a reference, the program takes approximately 1.5 hours to construct this data.
This is a one-time cost for every new input `.xls` file, as the relations are stored in a separate csv file.

However, once constructed, the temporal relations of the given set of authors can be queried and visualized very quickly.

== Author network <author_network_limitation>
Authors are not added to the network if they do not have any collaborators.
This is a limitation in the way the temporal relation network is constructed, as it does not differentiate between authors with no collaborators and authors with no published works.

This restricts the minimum connections an author can have to 1, which may not be representative of the actual network as authors can publish papers with no collaborators.

```csv
# an author with no collaborators from the temporal relations file
498,Luciano Timb Barbosa,,,,,,,,,,,,,,,,,,,,,,,,,
```

= Notes

== Detailed run guide
A more detailed guide on how to run the program is located in `README.md`.

== Python bundling
The main program (`project.py`) is executable as a single file when all dependencies have been installed.
However, the file is bundled from multiple python source files in a way that makes it less readable.

The original source of the python program is located in the unzipped file in: `/code/networkscience/`.

== Pre-generated relations file
A pre-generated relations file from the sample input file is provided in the zip file: `temporal_rels.csv`.

#pagebreak()
#bibliography(("bib.bib","bib.yaml"))
