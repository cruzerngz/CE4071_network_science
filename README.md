# CE4071_network_science

## Bundling
```sh
make
```

Code lives in `./networkscience`.
[`pinliner`](https://github.com/Akrog/pinliner) (a python script bundler) is used to bundle all code into a single file.
Due to the way the bundler works, all top-level strings must be delimited with double quotes `"`. Nested strings can use single quotes and alternate as per.


## Python3 interpreter not found (windows)
```
error: no Python 3.x interpreter found
```

There may be cases in windows where the python3 alias is not bound, or bound to the App store.

Follow the instructions in [this link](https://stackoverflow.com/questions/58754860/cmd-opens-windows-store-when-i-type-python) and enable the `python3` alias for a python executable (e.g. `python3.11.exe` should be enabled).
