.DEFAULT_GOAL := help

PYTHONPATH=
SHELL=bash
VENV=.venv

# On Windows, `Scripts/` is used.
ifeq ($(OS),Windows_NT)
	VENV_BIN=$(VENV)/Scripts
else
	VENV_BIN=$(VENV)/bin
endif

.PHONY: deps
deps: ## Install dependencies
	python -m pip install --upgrade uv && \
	uv venv $(VENV) && \
	source $(VENV_BIN)/activate && \
	uv pip install -r dev-requirements.txt && \
	uv pip install -e .

.PHONY: test	
test: ## Run tests
	cd python/tests && \
	../../$(VENV_BIN)/pytest

.PHONY: lint
lint: ## Run linting checks
	$(VENV_BIN)/ruff check .
	$(VENV_BIN)/ruff format . --check

.PHONY: fmt
fmt:  ## Format the code
	$(VENV_BIN)/ruff check . --fix  && \
	$(VENV_BIN)/ruff format .

.PHONY: type-check
type-check: ## Run type checking
	$(VENV_BIN)/pyright

.PHONY: help
help:  ## Display this help screen
	@echo -e "\033[1mAvailable commands:\033[0m"
	@grep -E '^[a-z.A-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2}' | sort