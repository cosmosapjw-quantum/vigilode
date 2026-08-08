PYTHON ?= python3

.PHONY: harness-init harness-check tree

harness-init:
	$(PYTHON) tools/init_harness.py

harness-check:
	$(PYTHON) tools/validate_harness.py

tree:
	find . -maxdepth 4 -type f | sort
