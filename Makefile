.DEFAULT_GOAL := help

# ==============================================================================
# VARIABLES
# ==============================================================================
IMAGE_NAME := jaeaeich/metis
VERSION ?= $(shell git describe --tags --always --dirty)
VCS_REF ?= $(shell git rev-parse --short HEAD)

# ==============================================================================
# HELP
# ==============================================================================
.PHONY: help
help:
	@echo "\nUsage: make [target] ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++\n"
	@echo "Available targets:\n"

	@echo "Code Generation ---------------------------------------------------------------"
	@echo "  \033[1m\033[35moapi\033[0m \033[37m(o)\033[0m: \033[36mGenerate API code from OpenAPI spec.\033[0m"
	@echo "  \033[1m\033[35mgp\033[0m: \033[36mGenerate protobuf code.\033[0m\n"

	@echo "Code Quality ------------------------------------------------------------------"
	@echo "  \033[1m\033[35mprecommit-check\033[0m \033[37m(pc)\033[0m: \033[36mRun all pre-commit checks.\033[0m\n"

	@echo "Dependencies ------------------------------------------------------------------"
	@echo "  \033[1m\033[35mtidy\033[0m \033[37m(t)\033[0m: \033[36mManages Go module dependencies.\033[0m\n"

	@echo "Development -------------------------------------------------------------------"
	@echo "  \033[1m\033[35mbuild\033[0m \033[37m(b)\033[0m: \033[36mBuild the Go application binary.\033[0m"
	@echo "  \033[1m\033[35mdev\033[0m \033[37m(d)\033[0m: \033[36mRun the application with hot-reload.\033[0m\n"

	@echo "Docker ------------------------------------------------------------------------"
	@echo "  \033[1m\033[35mbi\033[0m: \033[36mBuild the default distroless Docker image.\033[0m"
	@echo "  \033[1m\033[35mbia\033[0m: \033[36mBuild the alpine Docker image.\033[0m"
	@echo "  \033[1m\033[35mbid\033[0m: \033[36mBuild the dev Docker image.\033[0m\n"

	@echo "Kubernetes --------------------------------------------------------------------"
	@echo "  \033[1m\033[35mcreate-plugin-configmap\033[0m \033[37m(cpc)\033[0m: \033[36mCreates/updates the plugin configmap in Kubernetes.\033[0m\n"


# ==============================================================================
# DEVELOPMENT
# ==============================================================================
.PHONY: build b
build:
	go build -o bin/metis ./cmd

b: build

.PHONY: dev d
dev:
	go run -modfile=tools.mod github.com/air-verse/air

d: dev


# ==============================================================================
# DOCKER
# ==============================================================================
.PHONY: bi
bi:
	@echo "Building distroless image..."
	@docker build \
		--build-arg VERSION=$(VERSION) \
		--build-arg VCS_REF=$(VCS_REF) \
		--target distroless \
		-t $(IMAGE_NAME):$(VERSION) \
		-t $(IMAGE_NAME):latest \
		-f deployment/images/Dockerfile .
	@echo "Docker image name: $(IMAGE_NAME):$(VERSION), $(IMAGE_NAME):latest"

.PHONY: bia
bia:
	@echo "Building alpine image..."
	@docker build \
		--build-arg VERSION=$(VERSION) \
		--build-arg VCS_REF=$(VCS_REF) \
		--target alpine \
		-t $(IMAGE_NAME):alpine-$(VERSION) \
		-t $(IMAGE_NAME):alpine \
		-f deployment/images/Dockerfile .
	@echo "Docker image name: $(IMAGE_NAME):alpine-$(VERSION), $(IMAGE_NAME):alpine"

.PHONY: bid
bid:
	@echo "Building dev image..."
	@docker build \
		--build-arg VERSION=$(VERSION) \
		--build-arg VCS_REF=$(VCS_REF) \
		--target dev \
		-t $(IMAGE_NAME):dev-$(VERSION) \
		-t $(IMAGE_NAME):dev \
		-f deployment/images/Dockerfile .
	@echo "Docker image name: $(IMAGE_NAME):dev-$(VERSION), $(IMAGE_NAME):dev"

# ==============================================================================
# DEPENDENCIES
# ==============================================================================
.PHONY: tidy t
tidy:
	@echo "Tidying up..."
	go mod tidy
	go mod tidy -modfile=tools.mod
	go mod verify -modfile=tools.mod
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
		internal/api/spec/3a832ab.wes.yaml

	@echo "  - Generating server (fiber, strict-server)..."
	oapi-codegen \
		--import-mapping "openapi/3a832ab.spec.yaml:mytool/internal/api/generated,./service-info.yaml:api" \
		-generate fiber,strict-server \
		-package api \
		-o internal/api/generated/server.gen.go \
		internal/api/spec/3a832ab.wes.yaml

	@echo "API code generated successfully!"

o: oapi

.PHONY: generate-proto gp
generate-proto:
	@echo "Generating protobuf code..."
	protoc --go_out=. --go_opt=paths=source_relative \
		--go-grpc_out=. --go-grpc_opt=paths=source_relative \
		internal/metel/proto/plugin.proto
	@echo "Protobuf code generated successfully!"

gp: generate-proto


# ==============================================================================
# CODE QUALITY
# ==============================================================================
.PHONY: precommit-check pc
precommit-check:
	@echo "\nRunning pre-commit checks..."
	@pre-commit run --all-files

pc: precommit-check

.PHONY: format-lint fl
format-lint:
	@echo "\nRunning linter and formatter using golangci-lint..."
	@golangci-lint run --fix ./...

fl: format-lint


# ==============================================================================
# KUBERNETES
# ==============================================================================
.PHONY: create-plugin-configmap
create-plugin-configmap:
	@echo "Creating plugin configmap..."
	@kubectl create configmap metis-plugin-configmap --from-file=$(HOME)/.metis/plugins.yaml -n metis --dry-run=client -o yaml | kubectl apply -f -
	@echo "Plugin configmap created successfully!"

cpc: create-plugin-configmap
