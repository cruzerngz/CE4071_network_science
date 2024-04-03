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
    authors: typing.Optional[str]
    citations: typing.Optional[str]
    publisher: typing.Optional[str]
    school: typing.Optional[str]

    # def __repr__(self):
    #     dict(self)

class PersonRecord(object):
    """Record of a person in the database."""

    name: str
    profile: str
    aliases: str

    # def __repr__(self):
    #     dict(self)

class DblpRecordIter(object): ...
class PersonRecordIter(object): ...

def hello_world(): ...

def init_from_sqlite(path: str = None):
    """initialize the database from the given sqlite file.

    If no file is provided, then `dblp.sqlite` is used.`
    """

def init_from_xml(path: str = None):
    """initialize the database from the given xml file.

    If no file is provided, then `dblp.xml.gz` is used.
    """

def query_persons_table(constraints: str) -> list[PersonRecord]:
    """Query the persons table with the given constraints."""

def query_publications_table(constraints: str) -> list[DblpRecord]:
    """Query the publications table with the given constraints."""
