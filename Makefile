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
	python -m pip install --upgrade uv

	@if [ ! -d "$(VENV)" ]; then \
		uv venv $(VENV); \
		echo "Virtual environment created at $(VENV)"; \
	else \
		echo "Virtual environment already exists at $(VENV)"; \
	fi

	source $(VENV_BIN)/activate && \
	uv pip install -e ".[dev]"

	@unset CONDA_PREFIX && \
	source $(VENV_BIN)/activate && \
	maturin develop --profile release -E dev


.PHONY: install
install: ##  Install the crate as module in the current virtualenv
	maturin develop --release -E dev


.PHONY: test
test: ## Run tests
	cd python/tests && \
	../../$(VENV_BIN)/pytest


.PHONY: lint fmt lint-rust lint-python fmt-rust fmt-python

lint: lint-rust lint-python  ## Run linting checks for Rust and Python code
fmt: fmt-rust fmt-python  ## Format Rust and Python code


fmt-python: ## Format Python code
	$(VENV_BIN)/ruff check . --fix
	$(VENV_BIN)/ruff format .


fmt-rust: ## Format Rust code
	cargo fmt --all
	cargo clippy --fix --allow-dirty --allow-staged


lint-python: ## Lint Python code
	$(VENV_BIN)/ruff check .
	$(VENV_BIN)/ruff format . --check


lint-rust: ## Lint Rust code
	cargo fmt --all --check
	cargo clippy


.PHONY: type-check
type-check: ## Run type checking
	$(VENV_BIN)/pyright


.PHONY: docs
docs: ## Develop on docs locally
	cd docs && npx mintlify dev


.PHONY: help
help:  ## Display this help screen
	@echo -e "\033[1mAvailable commands:\033[0m"
	@grep -E '^[a-z.A-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2}' | sort
