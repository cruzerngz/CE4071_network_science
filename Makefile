
default: bundle

venv:
	python3 -m venv venv

install:
	pip3 install -r requierements.txt

bundle: clean
	pinliner networkscience -o project.py

dblp-lib:
	cd dblp && maturin develop --release

clean:
	rm -f project.py

copy: bundle
	yes | rm -rf ./project

	mkdir -p project

	git clone . project/code
	cp project/code/README.md project/
	cp project.py project/

report: copy
	typst compile report/main.typ project/report.pdf

submit: bundle report
	zip -r project.zip project
