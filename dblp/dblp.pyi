# definitions in this file must match with lib.rs

import typing

class PublicationRecord(object): ...

class DblpRecord(object):
    """Record of an article in the database."""

    record: PublicationRecord
    key: str
    mdate: typing.Optional[str]
    publtype: typing.Optional[str]
    year: typing.Optional[int]
    # access authors through the `authors` method
    # authors: typing.Optional[str]
    citations: typing.Optional[str]
    publisher: typing.Optional[str]
    school: typing.Optional[str]

    def authors(self) -> list[str]:
        """Return the list of authors associated with this publication."""

    # def x(self) -> bool:
    #     init_from_sqlite()

class PersonRecord(object):
    """Record of a person in the database."""

    name: str
    profile: str
    aliases: str

class DblpRecordIter(object): ...
class PersonRecordIter(object): ...

def hello_world(): ...

def init_from_sqlite(path: str = None):
    """initialize the database from the given sqlite file.

    If no file is provided, then `dblp.sqlite` is used.`
    """

def init_from_xml(path: str = None):
    """initialize the database from the given xml file.

    If no file is provided, then `dblp.xml` or `dblp.xml.gz` is used.
    """

def query_persons_table(constraints: str) -> list[PersonRecord]:
    """Perform a raw query on the persons table with the given constraints.

    E.g. `WHERE true = true` to get all records.

    E.g. `LIMIT 10` to get the first 10 records.
    """

def query_publications_table(constraints: str) -> list[DblpRecord]:
    """Perform a raw query on the publications table with the given constraints.

    E.g. `WHERE true = true` to get all records.

    E.g. `LIMIT 10` to get the first 10 records.
    """

def query_person(name: str, limit: int = None) -> list[PersonRecord]:
    """Query the persons table with the given name.

    If an exact match is not found, a search over aliases is performed.
    """

def query_publication(name: str, limit: int = None) -> list[DblpRecord]:
    """Query the publications table with the given name."""

def query_person_publications(name: str, max_year: int = None, limit: int = None) -> list[DblpRecord]:
    """Search for all publications from a specific author."""
