.DEFAULT_GOAL := help
.PHONY: help setup install lint format package

PYTHON_VERSION = 3.8.11

define PRINT_HELP_PYSCRIPT
import re, sys

for line in sys.stdin:
	match = re.match(r'^([a-zA-Z_-]+):.*?## (.*)$$', line)
	if match:
		target, help = match.groups()
		print("%-20s %s" % (target, help))
endef
export PRINT_HELP_PYSCRIPT

help:
	@python3 -c "$$PRINT_HELP_PYSCRIPT" < $(MAKEFILE_LIST)

setup: ## setup python with pyenv and poetry
	@poetry env use $(shell python -c "import sys; print(sys.executable)")
	poetry config virtualenvs.in-project true
	poetry shell 

install: setup ## install dependencies
	poetry install

format: install ## format package and test files using black
	poetry run black src/

package: install lint format ## package the project
	pyinstaller src/bash_command_finder.py