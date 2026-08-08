PYTHON ?= python3

.PHONY: init validate tree

init:
	$(PYTHON) tools/init_workspace.py

validate:
	$(PYTHON) tools/validate_workspace.py

tree:
	find . -maxdepth 3 -type f | sort
