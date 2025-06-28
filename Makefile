.DEFAULT_GOAL := help

# ==============================================================================
# HELP
# ==============================================================================
.PHONY: help
help:
	@echo "\nUsage: make [target] ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++\n"
	@echo "Available targets:\n"

	@echo "Code Generation ---------------------------------------------------------------"
	@echo "  \033[1m\033[35moapi\033[0m \033[37m(o)\033[0m: \033[36mGenerate API code from OpenAPI spec.\033[0m\n"

	@echo "Code Quality ------------------------------------------------------------------"
	@echo "  \033[1m\033[35mprecommit-check\033[0m \033[37m(pc)\033[0m: \033[36mRun all pre-commit checks.\033[0m\n"

	@echo "Dependencies ------------------------------------------------------------------"
	@echo "  \033[1m\033[35mtidy\033[0m \033[37m(t)\033[0m: \033[36mManages Go module dependencies.\033[0m\n"

	@echo "Development -------------------------------------------------------------------"
	@echo "  \033[1m\033[35mbuild\033[0m \033[37m(b)\033[0m: \033[36mBuild the Go application binary.\033[0m\n"


# ==============================================================================
# DEVELOPMENT
# ==============================================================================
.PHONY: build b
build:
	go build -o bin/metis ./cmd

b: build


# ==============================================================================
# DEPENDENCIES
# ==============================================================================
.PHONY: tidy t
tidy:
	@echo "Tidying up..."
	go mod tidy
	go mod verify
	@echo "Tidying up completed!"

t: tidy


# ==============================================================================
# CODE GENERATION
# ==============================================================================
.PHONY: oapi o
oapi:
	@echo "Generating API code..."

	@echo "  - Generating types..."
	oapi-codegen \
		-generate types \
		-package api \
		-o internal/api/generated/models.gen.go \
		spec/3a832ab.wes.yaml

	@echo "  - Generating server (fiber, strict-server)..."
	oapi-codegen \
		--import-mapping "openapi/3a832ab.spec.yaml:mytool/internal/api/generated,./service-info.yaml:api" \
		-generate fiber,strict-server \
		-package api \
		-o internal/api/generated/server.gen.go \
		spec/3a832ab.wes.yaml

	@echo "API code generated successfully!"

o: oapi


# ==============================================================================
# CODE QUALITY
# ==============================================================================
.PHONY: precommit-check pc
precommit-check:
	@echo "\nRunning pre-commit checks +++++++++++++++++++++++++++++++++++++++++++++++++++++\n"
	@pre-commit run --all-files

pc: precommit-check
