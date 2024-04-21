# CE4071_network_science

## Prerequisites
- python >= 3.10
<!-- - venv -->
<!-- - [venv dependencies](./requirements.txt) -->
- [cargo](https://rustup.rs)

## Bundling
```sh
make
```

Code lives in `./networkscience`.
[`pinliner`](https://github.com/Akrog/pinliner) (a python script bundler) is used to bundle all code into a single file.
Due to the way the bundler works, all top-level strings must be delimited with double quotes `"`. Nested strings can use single quotes and alternate as per.

## Step-by-step guide
1. Ensure that [prerequisites](#prerequisites) are met
2. Install a virtualenv in the current directory
    ```
    python3 -m venv .venv
    ```

3. Install requirements inside the virtual environment
    ```sh
    . .venv/bin/activate # unix
    pip3 install -r requirements.txt
    ```

4. Compile and install `dblp` into virtual environment
    ```sh
    cd dblp/
    maturin develop --release
    ```

5. Bundle python package into a single file
    ```sh
    # if you can't run this makefile,
    # copy and paste target commands from the makefile.
    make
    ```

6. Download the dblp dataset from: https://dblp.uni-trier.de/xml/dblp.xml.gz
7. Run the program. Each stage writes some output to disk so that execution can resume from that save point
    ```sh
    # stage 1: convert xml data to sqlite database
    python3 project.py --xml dblp.xml.gz

    # stage 2: use existing sqlite database. Defaults to 'dblp.sqlite' if not set
    python3 project.py --sqlite dblp.sqlite

    # stage 3: associate raw input data to authors in database
    python3 project.py --xls DataScientists.xls

    # stage 4: generate the temporal relations between filtered authors (this takes a while)
    python3 project.py --csv filtered.csv

    # stage 5: generate visualisations
    python3 project.py --relations temporal_rels.csv
    ```

## Python3 interpreter not found (windows)
```
error: no Python 3.x interpreter found
```

There may be cases in windows where the python3 alias is not bound, or bound to the App store.

Follow the instructions in [this link](https://stackoverflow.com/questions/58754860/cmd-opens-windows-store-when-i-type-python) and enable the `python3` alias for a python executable (e.g. `python3.11.exe` should be enabled).

