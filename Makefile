# Makefile for the Textrynum Project

# ANSI color codes
BLUE := \033[1;34m
GREEN := \033[1;32m
YELLOW := \033[1;33m
RED := \033[1;31m
CYAN := \033[1;36m
RESET := \033[0m

# Variables
PROJECT_NAME := Textrynum
CODE_NAME := textrynum
BIN_DIR := ./bin
MODE := debug
TARGET := ./target/$(MODE)
GIT_COMMIT := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH := $(shell git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
BUILD_TIME := $(shell date -u '+%Y-%m-%dT%H:%M:%SZ')
RUST_VERSION := $(shell rustc --version 2>/dev/null || echo "unknown")

# Git remotes to push to
GIT_REMOTES := macpro github codeberg
REMOTE_macpro := ssh://macpro.local:23231/oxur/$(CODE_NAME).git
REMOTE_github := git@github.com:oxur/$(CODE_NAME).git
REMOTE_codeberg := ssh://git@codeberg.org/oxur/$(CODE_NAME).git

# Default target
.DEFAULT_GOAL := help

# Help target
.PHONY: help
help:
	@echo ""
	@echo "$(CYAN)╔══════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(CYAN)║$(RESET) $(BLUE)$(PROJECT_NAME) Build System$(RESET)                                   $(CYAN)║$(RESET)"
	@echo "$(CYAN)╚══════════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(GREEN)Building:$(RESET)"
	@echo "  $(YELLOW)make build$(RESET)            - Build all binaries ($(BINARIES))"
	@echo "  $(YELLOW)make build-release$(RESET)    - Build optimized release binaries"
	@echo "  $(YELLOW)make build MODE=release$(RESET) - Build with custom mode"
	@echo ""
	@echo "$(GREEN)Testing & Quality:$(RESET)"
	@echo "  $(YELLOW)make test$(RESET)             - Run all tests"
	@echo "  $(YELLOW)make lint$(RESET)             - Run clippy and format check"
	@echo "  $(YELLOW)make format$(RESET)           - Format all code with rustfmt"
	@echo "  $(YELLOW)make coverage$(RESET)         - Generate test coverage report"
	@echo "  $(YELLOW)make check$(RESET)            - Build + lint + test"
	@echo "  $(YELLOW)make check-all$(RESET)        - Build + lint + coverage"
	@echo ""
	@echo "$(GREEN)Cleaning:$(RESET)"
	@echo "  $(YELLOW)make clean$(RESET)            - Clean bin directory"
	@echo "  $(YELLOW)make clean-all$(RESET)        - Full clean (cargo clean)"
	@echo ""
	@echo "$(GREEN)Utilities:$(RESET)"
	@echo "  $(YELLOW)make remotes$(RESET)          - Set up git remotes for private hosting, Codeberg and Github"
	@echo "  $(YELLOW)make push$(RESET)             - Pushes to Codeberg and Github"
	@echo "  $(YELLOW)make publish$(RESET)          - WIP: Publishes all crates to crates.io"
	@echo "  $(YELLOW)make tracked-files$(RESET)    - Save list of tracked files"
	@echo ""
	@echo "$(GREEN)Information:$(RESET)"
	@echo "  $(YELLOW)make info$(RESET)             - Show build information"
	@echo "  $(YELLOW)make check-tools$(RESET)      - Verify required tools are installed"
	@echo ""
	@echo "$(CYAN)Current status:$(RESET) Branch: $(GIT_BRANCH) | Commit: $(GIT_COMMIT)"
	@echo ""

# Info target
.PHONY: info
info:
	@echo ""
	@echo "$(CYAN)╔══════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(CYAN)║$(RESET)  $(BLUE)Build Information$(RESET)                                       $(CYAN)║$(RESET)"
	@echo "$(CYAN)╚══════════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(GREEN)Project:$(RESET)"
	@echo "  Name:           $(PROJECT_NAME)"
	@echo "  Build Mode:     $(MODE)"
	@echo "  Build Time:     $(BUILD_TIME)"
	@echo ""
	@echo "$(GREEN)Paths:$(RESET)"
	@echo "  Binary Dir:     $(BIN_DIR)/"
	@echo "  Target Dir:     $(TARGET)/"
	@echo "  Workspace:      $$(pwd)"
	@echo ""
	@echo "$(GREEN)Git:$(RESET)"
	@echo "  Branch:         $(GIT_BRANCH)"
	@echo "  Commit:         $(GIT_COMMIT)"
	@echo ""
	@echo "$(GREEN)Tools:$(RESET)"
	@echo "  Rust:           $(RUST_VERSION)"
	@echo "  Cargo:          $$(cargo --version 2>/dev/null || echo 'not found')"
	@echo "  Rustfmt:        $$(rustfmt --version 2>/dev/null || echo 'not found')"
	@echo "  Clippy:         $$(cargo clippy --version 2>/dev/null || echo 'not found')"
	@echo ""
	@echo "$(GREEN)Binaries:$(RESET)"
	@for bin in $(BINARIES); do \
		if [ -f $(BIN_DIR)/$$bin ]; then \
			echo "  $$bin:          $(GREEN)✓ installed$(RESET)"; \
		else \
			echo "  $$bin:          $(RED)✗ not built$(RESET)"; \
		fi; \
	done
	@echo ""

# Check tools target
.PHONY: check-tools
check-tools:
	@echo "$(BLUE)Checking for required tools...$(RESET)"
	@command -v rustc >/dev/null 2>&1 && echo "$(GREEN)✓ rustc found (version: $$(rustc --version))$(RESET)" || echo "$(RED)✗ rustc not found$(RESET)"
	@command -v cargo >/dev/null 2>&1 && echo "$(GREEN)✓ cargo found (version: $$(cargo --version))$(RESET)" || echo "$(RED)✗ cargo not found$(RESET)"
	@command -v rustfmt >/dev/null 2>&1 && echo "$(GREEN)✓ rustfmt found$(RESET)" || echo "$(RED)✗ rustfmt not found (install: rustup component add rustfmt)$(RESET)"
	@cargo clippy --version >/dev/null 2>&1 && echo "$(GREEN)✓ clippy found$(RESET)" || echo "$(RED)✗ clippy not found (install: rustup component add clippy)$(RESET)"
	@cargo llvm-cov --version >/dev/null 2>&1 && echo "$(GREEN)✓ llvm-cov found$(RESET)" || echo "$(RED)✗ llvm-cov not found (install: cargo install cargo-llvm-cov)$(RESET)"
	@command -v git >/dev/null 2>&1 && echo "$(GREEN)✓ git found$(RESET)" || echo "$(RED)✗ git not found$(RESET)"
	@test -f Cargo.toml && echo "$(GREEN)✓ Cargo.toml found$(RESET)" || echo "$(RED)✗ Cargo.toml not found$(RESET)"

# Build directory creation
$(BIN_DIR):
	@echo "$(BLUE)Creating bin directory...$(RESET)"
	@mkdir -p $(BIN_DIR)
	@echo "$(GREEN)✓ Directory created$(RESET)"

# Build targets
.PHONY: build
build: clean $(BIN_DIR)
	@echo "$(BLUE)Building $(PROJECT_NAME) in $(MODE) mode...$(RESET)"
	@echo "$(CYAN)• Compiling workspace...$(RESET)"
	@if [ "$(MODE)" = "release" ]; then \
		cargo build --release; \
	else \
		cargo build; \
	fi
	@echo "$(CYAN)• Copying binaries to $(BIN_DIR)/$(RESET)"
	@for bin in $(BINARIES); do \
		if [ -f $(TARGET)/$$bin ]; then \
			cp $(TARGET)/$$bin $(BIN_DIR)/$$bin; \
			echo "  $(GREEN)✓$(RESET) $$bin"; \
		else \
			echo "  $(YELLOW)⚠$(RESET) $$bin not found, skipping"; \
		fi; \
	done
	@echo "$(GREEN)✓ Build complete$(RESET)"
	@echo "$(CYAN)→ Binaries available in $(BIN_DIR)/$(RESET)"

.PHONY: build-release
build-release: MODE = release
build-release: TARGET = ./target/$(MODE)
build-release: ODM_TARGET = $(OXUR_WORKSPACE)/target/$(MODE)
build-release: clean $(BIN_DIR)
	@echo "$(BLUE)Building $(PROJECT_NAME) in release mode...$(RESET)"
	@echo "$(CYAN)• Compiling optimized workspace...$(RESET)"
	@cargo build --release
	@echo "$(CYAN)• Copying binaries to $(BIN_DIR)/$(RESET)"
	@for bin in $(BINARIES); do \
		if [ -f $(TARGET)/$$bin ]; then \
			cp $(TARGET)/$$bin $(BIN_DIR)/$$bin; \
			echo "  $(GREEN)✓$(RESET) $$bin (size: $$(du -h $(BIN_DIR)/$$bin | cut -f1))"; \
		else \
			echo "  $(YELLOW)⚠$(RESET) $$bin not found, skipping"; \
		fi; \
	done
	@echo "$(GREEN)✓ Release build complete$(RESET)"
	@echo "$(CYAN)→ Optimized binaries in $(BIN_DIR)/$(RESET)"

# Cleaning targets
.PHONY: clean
clean:
	@echo "$(BLUE)Cleaning bin directory...$(RESET)"
	@rm -rf $(BIN_DIR)
	@echo "$(GREEN)✓ Clean complete$(RESET)"

.PHONY: clean-all
clean-all: clean
	@echo "$(BLUE)Performing full cargo clean...$(RESET)"
	@cargo clean
	@echo "$(GREEN)✓ Full clean complete$(RESET)"

# Testing & Quality targets
.PHONY: test
test:
	@echo "$(BLUE)Running tests...$(RESET)"
	@echo "$(CYAN)• Running all workspace tests...$(RESET)"
	@cargo test --all-features --workspace
	@echo "$(GREEN)✓ All tests passed$(RESET)"

.PHONY: lint
lint:
	@echo "$(BLUE)Running linter checks...$(RESET)"
	@echo "$(CYAN)• Running clippy...$(RESET)"
	@cargo clippy --all-features --workspace -- -D warnings
	@echo "$(GREEN)✓ Clippy passed$(RESET)"
	@echo "$(CYAN)• Checking code formatting...$(RESET)"
	@cargo fmt --all -- --check
	@echo "$(GREEN)✓ Format check passed$(RESET)"

.PHONY: format
format:
	@echo "$(BLUE)Formatting code...$(RESET)"
	@echo "$(CYAN)• Running rustfmt on all files...$(RESET)"
	@cargo fmt --all
	@echo "$(GREEN)✓ Code formatted$(RESET)"

.PHONY: coverage
coverage:
	@echo "$(BLUE)Generating test coverage report...$(RESET)"
	@echo "$(CYAN)• Running tests with coverage (all workspace crates)...$(RESET)"
	@cargo llvm-cov --lib --workspace
	@echo "$(GREEN)✓ Coverage report generated$(RESET)"
	@echo "$(YELLOW)→ For detailed HTML report, run: make coverage-html$(RESET)"

.PHONY: coverage-html
coverage-html:
	@echo "$(BLUE)Generating HTML coverage report...$(RESET)"
	@echo "$(CYAN)• Running tests with coverage (all workspace crates)...$(RESET)"
	@cargo llvm-cov --html --lib --workspace
	@echo "$(GREEN)✓ HTML coverage report generated$(RESET)"
	@echo "$(CYAN)→ Report: target/llvm-cov/html/index.html$(RESET)"

# Combined check targets
.PHONY: check
check: build lint test
	@echo ""
	@echo "$(GREEN)✓ All checks passed (build + lint + test)$(RESET)"
	@echo ""

.PHONY: check-all
check-all: build lint coverage
	@echo ""
	@echo "$(GREEN)✓ Full validation complete (build + lint + coverage)$(RESET)"
	@echo ""

# Utility targets
.PHONY: tracked-files
tracked-files:
	@echo "$(BLUE)Saving tracked files list...$(RESET)"
	@mkdir -p $(TARGET)
	@git ls-files > $(TARGET)/git-tracked-files.txt
	@echo "$(GREEN)✓ Tracked files saved to $(TARGET)/git-tracked-files.txt$(RESET)"
	@echo "$(CYAN)• Total files: $$(wc -l < $(TARGET)/git-tracked-files.txt)$(RESET)"

# Ensure cargo-binstall is available for fast tool installation
.PHONY: ensure-binstall
ensure-binstall:
	@if ! cargo binstall --help >/dev/null 2>&1; then \
		echo "$(YELLOW)→ Installing cargo-binstall...$(RESET)"; \
		curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash; \
	fi

.PHONY: check-deps
check-deps: ensure-binstall
	@echo "$(BLUE)Checking for outdated dependencies...$(RESET)"
	@if ! cargo upgrade --help >/dev/null 2>&1; then \
		echo "$(YELLOW)→ Installing cargo-edit...$(RESET)"; \
		cargo binstall -y --force cargo-edit; \
	fi
	@OUTPUT=$$(cargo upgrade --dry-run --incompatible 2>&1); \
	if ! echo "$$OUTPUT" | grep -q "aborting upgrade due to dry run"; then \
		echo "$(RED)✗ cargo upgrade --dry-run failed:$(RESET)"; \
		echo "$$OUTPUT"; \
		exit 1; \
	fi; \
	UPDATES=$$(echo "$$OUTPUT" | awk '/^[a-z][a-z0-9_-]/ && !/^name / { \
		split($$2, a, "."); split($$4, b, "."); \
		if (a[1] == b[1] && (b[2]+0 > a[2]+0 || (b[2]+0 == a[2]+0 && b[3]+0 >= a[3]+0))) print \
	}' | sort -t' ' -k1,1 -u); \
	MAJOR=$$(echo "$$OUTPUT" | awk '/^[a-z][a-z0-9_-]/ && !/^name / { \
		split($$2, a, "."); split($$5, b, "."); \
		if (a[1] != b[1]) print \
	}' | sort -t' ' -k1,1 -u | wc -l | tr -d ' '); \
	if [ -n "$$UPDATES" ]; then \
		printf "%-20s %-10s %-10s\n" "name" "current" "latest"; \
		printf "%-20s %-10s %-10s\n" "====" "=======" "======"; \
		echo "$$UPDATES" | awk '{ printf "%-20s %-10s %-10s\n", $$1, $$2, $$4 }'; \
		echo ""; \
		echo "$(RED)✗ Dependency updates available$(RESET)"; \
		echo "$(YELLOW)→ Run 'make deps' to update$(RESET)"; \
		if [ "$$MAJOR" -gt 0 ]; then \
			echo "$(CYAN)  ($$MAJOR major-version updates not shown)$(RESET)"; \
		fi; \
		exit 1; \
	else \
		echo "$(GREEN)✓ All dependencies up to date$(RESET)"; \
		if [ "$$MAJOR" -gt 0 ]; then \
			echo "$(CYAN)  ($$MAJOR major-version updates available, not shown)$(RESET)"; \
		fi; \
	fi
.PHONY: check-deps-cicd
check-deps-cicd: ensure-binstall
	@echo "$(BLUE)Checking for outdated dependencies (compatible only)...$(RESET)"
	@if ! cargo upgrade --help >/dev/null 2>&1; then \
		echo "$(YELLOW)→ Installing cargo-edit...$(RESET)"; \
		cargo binstall -y --force cargo-edit; \
	fi
	@OUTPUT=$$(cargo upgrade --dry-run 2>&1); \
	if ! echo "$$OUTPUT" | grep -q "aborting upgrade due to dry run"; then \
		echo "$(RED)✗ cargo upgrade --dry-run failed:$(RESET)"; \
		echo "$$OUTPUT"; \
		exit 1; \
	fi; \
	UPDATES=$$(echo "$$OUTPUT" | awk '/^[a-z][a-z0-9_-]/ && !/^name / && !/^note:/ && !/^warning:/ { print }' | sort -t' ' -k1,1 -u); \
	if [ -n "$$UPDATES" ]; then \
		printf "%-20s %-10s %-10s\n" "name" "current" "latest"; \
		printf "%-20s %-10s %-10s\n" "====" "=======" "======"; \
		echo "$$UPDATES" | awk '{ printf "%-20s %-10s %-10s\n", $$1, $$2, $$4 }'; \
		echo ""; \
		echo "$(RED)✗ Compatible dependency updates available$(RESET)"; \
		echo "$(YELLOW)→ Run 'make deps' to update$(RESET)"; \
		exit 1; \
	else \
		echo "$(GREEN)✓ All dependencies up to date$(RESET)"; \
	fi

.PHONY: deps
deps: ensure-binstall
	@echo "$(BLUE)Updating dependencies ...$(RESET)"
	@if ! cargo upgrade --help >/dev/null 2>&1; then \
		echo "$(YELLOW)→ Installing cargo-edit...$(RESET)"; \
		cargo binstall -y --force cargo-edit; \
	fi
	@cargo upgrade
	@echo "$(GREEN)✓ Cargo deps upgraded$(RESET)"

.PHONY: remotes
remotes:
	@echo "$(BLUE)Configuring git remotes...$(RESET)"
	@for remote in $(GIT_REMOTES); do \
		case $$remote in \
			macpro)   url="$(REMOTE_macpro)"   ;; \
			github)   url="$(REMOTE_github)"   ;; \
			codeberg) url="$(REMOTE_codeberg)" ;; \
		esac; \
		if git remote get-url $$remote >/dev/null 2>&1; then \
			echo "  $(YELLOW)⊙$(RESET) $$remote already exists ($$url)"; \
		else \
			git remote add $$remote $$url; \
			echo "  $(GREEN)✓$(RESET) Added $$remote → $$url"; \
		fi; \
	done
	@echo "$(GREEN)✓ Remotes configured$(RESET)"

push:
	@echo "$(BLUE)Pushing changes ...$(RESET)"
	@for remote in $(GIT_REMOTES); do \
		echo "$(CYAN)• $$remote:$(RESET)"; \
		git push $$remote main && git push $$remote --tags; \
		echo "$(GREEN)✓ Pushed$(RESET)"; \
	done

# Crates in dependency order (leaf crates first, dependent crates later)
# ECL crates: design, core -> steps -> workflows -> cli -> ecl
# Fabryk auth: auth, auth-mcp -> auth-google
# Fabryk crates: core -> (content, acl, fts, mcp) -> (graph, vector) ->
#                (mcp-content, mcp-fts, mcp-graph) -> cli -> fabryk
PUBLISH_ORDER := ecl-core ecl-steps ecl-workflows ecl-cli ecl \
                 fabryk-auth fabryk-auth-mcp fabryk-auth-google \
                 fabryk-core fabryk-content fabryk-acl fabryk-fts fabryk-mcp \
                 fabryk-graph fabryk-vector \
                 fabryk-mcp-content fabryk-mcp-fts fabryk-mcp-graph \
                 fabryk-cli fabryk
# crates.io rate limit delay (seconds)
PUBLISH_DELAY := 372
.PHONY: publish
publish:
	@echo ""
	@echo "$(CYAN)╔══════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(CYAN)║$(RESET) $(BLUE)Publishing $(PROJECT_NAME) Crates to crates.io$(RESET)                 $(CYAN)║$(RESET)"
	@echo "$(CYAN)╚══════════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(YELLOW)⚠ This will publish all crates in dependency order$(RESET)"
	@echo "$(YELLOW)⚠ Ensure all tests pass and versions are updated$(RESET)"
	@echo ""
	@read -p "Continue? [y/N] " -n 1 -r; \
	echo; \
	if [[ ! $$REPLY =~ ^[Yy]$$ ]]; then \
		echo "$(RED)✗ Aborted$(RESET)"; \
		exit 1; \
	fi
	@echo ""
	@echo "$(BLUE)Publishing crates in dependency order...$(RESET)"
	@echo "$(YELLOW)Note: Publishing ~10 new crates/hour to avoid rate limits$(RESET)"
	@echo ""
	@for crate in $(PUBLISH_ORDER); do \
		echo ""; \
		echo "$(CYAN)• Publishing $$crate...$(RESET)"; \
		cd crates/$$crate && \
		output=$$(cargo publish 2>&1); \
		result=$$?; \
		cd ../..; \
		if [ $$result -eq 0 ]; then \
			echo "  $(GREEN)✓$(RESET) $$crate published successfully"; \
			echo "  $(YELLOW)→ Waiting 6 minutes for crates.io rate limit and index update...$(RESET)"; \
			sleep $(PUBLISH_DELAY); \
		elif echo "$$output" | grep -q "already exists"; then \
			echo "  $(YELLOW)⊙$(RESET) $$crate already published, skipping"; \
		elif echo "$$output" | grep -q "429 Too Many Requests"; then \
			echo "  $(YELLOW)⚠$(RESET) Rate limit hit for $$crate"; \
			retry_after=$$(echo "$$output" | sed -n 's/.*after \([^.]*\).*/\1/p' | head -1); \
			if [ -n "$$retry_after" ]; then \
				echo "  $(YELLOW)→$(RESET) Server says: retry after $$retry_after"; \
			fi; \
			echo "  $(YELLOW)→$(RESET) Tip: Email help@crates.io to request a limit increase"; \
			echo "  $(YELLOW)→$(RESET) Or wait and run: cd crates/$$crate && cargo publish"; \
			exit 1; \
		else \
			echo "  $(RED)✗$(RESET) Failed to publish $$crate"; \
			echo "$$output"; \
			exit 1; \
		fi; \
	done
	@echo ""
	@echo "$(GREEN)✓ All crates published successfully!$(RESET)"
	@echo ""

.PHONY: publish-dry-run
publish-dry-run:
	@echo ""
	@echo "$(CYAN)╔══════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(CYAN)║$(RESET) $(BLUE)Dry Run: Publishing $(PROJECT_NAME) Crates$(RESET)                     $(CYAN)║$(RESET)"
	@echo "$(CYAN)╚══════════════════════════════════════════════════════════╝$(RESET)"
	@echo ""
	@echo "$(BLUE)Publishing order (in dependency order):$(RESET)"
	@i=1; \
	for crate in $(PUBLISH_ORDER); do \
		echo "  $(YELLOW)$$i.$(RESET) $$crate"; \
		i=$$((i+1)); \
	done
	@echo ""
	@echo "$(BLUE)Verifying each crate can be packaged...$(RESET)"
	@for crate in $(PUBLISH_ORDER); do \
		echo ""; \
		echo "$(CYAN)• Packaging $$crate...$(RESET)"; \
		cd crates/$$crate && \
		cargo package --list > /dev/null 2>&1 && \
		cd ../..; \
		if [ $$? -eq 0 ]; then \
			echo "  $(GREEN)✓$(RESET) $$crate is ready for publishing"; \
		else \
			echo "  $(RED)✗$(RESET) $$crate failed validation"; \
			cd crates/$$crate && cargo package --list && cd ../..; \
			exit 1; \
		fi; \
	done
	@echo ""
	@echo "$(GREEN)✓ All crates ready for publishing!$(RESET)"
	@echo "$(CYAN)→ Run 'make publish' to publish to crates.io$(RESET)"
	@echo "$(CYAN)→ Or 'make publish-one CRATE=crate-name' to publish a single crate$(RESET)"
	@echo ""

.PHONY: publish-one
publish-one:
	@if [ -z "$(CRATE)" ]; then \
		echo "$(RED)Error: CRATE variable not set$(RESET)"; \
		echo "Usage: make publish-one CRATE=ecl-core"; \
		exit 1; \
	fi
	@echo ""
	@echo "$(CYAN)Publishing $(CRATE)...$(RESET)"
	@cd crates/$(CRATE) && cargo publish
	@echo ""
	@echo "$(GREEN)✓ Published $(CRATE)$(RESET)"
	@echo ""
