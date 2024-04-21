# CE4071_network_science

## Prerequisites
- python >= 3.10
- [cargo](https://rustup.rs)
- [pygraphviz](https://pygraphviz.github.io/documentation/stable/install.html)
- [WSL](https://learn.microsoft.com/en-us/windows/wsl/install) or a unix system

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

    # unix
    make dblp-lib
    ```

5. Bundle python package into a single file
    ```sh
    python3 -m pinliner networkscience -o project.py

    # unix
    make
    ```

6. Download the dblp dataset from: https://dblp.uni-trier.de/xml/dblp.xml.gz
7. Run the program. Each stage writes some output to disk so that execution can resume from that save point
    ```sh
    # stage 1: convert xml data to sqlite database
    python3 project.py --xml dblp.xml.gz # other flags required. use --help to view them

    # stage 1.5: use existing sqlite database. Defaults to 'dblp.sqlite' if not set
    python3 project.py --sqlite dblp.sqlite

    # stage 2: associate raw input data to authors in database
    # the following 2 lines are equivalent
    python3 project.py --xls DataScientists.xls
    python3 project.py --sqlite dblp.sqlite --xls DataScientists.xls

    # stage 3: generate the temporal relations between filtered authors (this takes a while)
    python3 project.py --csv filtered.csv

    # stage 4: generate visualisations (with optional output prefix for all generated files)
    python3 project.py --relations temporal_rels.csv --file-prefix "output/run_1"
    ```

## Python3 interpreter not found (windows)
```
error: no Python 3.x interpreter found
```

There may be cases in windows where the python3 alias is not bound, or bound to the App store.

Follow the instructions in [this link](https://stackoverflow.com/questions/58754860/cmd-opens-windows-store-when-i-type-python) and enable the `python3` alias for a python executable (e.g. `python3.11.exe` should be enabled).

