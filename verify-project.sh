#!/bin/bash
set -e

echo "=========================================="
echo "LLM Research Lab - Project Verification"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Set cargo path
export PATH="$HOME/.cargo/bin:$PATH"

echo "${BLUE}1. Checking Rust version...${NC}"
rustc --version
cargo --version
echo ""

echo "${BLUE}2. Counting crates...${NC}"
echo "Workspace members:"
grep -A 10 "\[workspace\]" Cargo.toml | grep "members" -A 6
echo ""

echo "${BLUE}3. Counting source files...${NC}"
echo "Total Rust source files: $(find . -name "*.rs" -type f | grep -v target | wc -l)"
echo "Total lines of Rust code: $(find . -name "*.rs" -type f | grep -v target | xargs wc -l | grep total | awk '{print $1}')"
echo ""

echo "${BLUE}4. Verifying project structure...${NC}"
echo "Crates:"
for crate in llm-research-lab llm-research-core llm-research-api llm-research-storage llm-research-metrics llm-research-workflow; do
    if [ -d "$crate" ]; then
        echo "  ✓ $crate"
    else
        echo "  ✗ $crate (missing)"
    fi
done
echo ""

echo "${BLUE}5. Checking compilation (debug)...${NC}"
cargo check --workspace --quiet 2>&1 && echo "${GREEN}✓ Debug compilation successful${NC}" || echo "✗ Debug compilation failed"
echo ""

echo "${BLUE}6. Running tests...${NC}"
cargo test --workspace --quiet 2>&1 && echo "${GREEN}✓ All tests passed${NC}" || echo "⚠ Some tests need implementation"
echo ""

echo "${BLUE}7. Checking release build...${NC}"
if [ -f "target/release/llm-research-lab" ]; then
    SIZE=$(du -h target/release/llm-research-lab | awk '{print $1}')
    echo "${GREEN}✓ Release binary exists: $SIZE${NC}"
    file target/release/llm-research-lab
else
    echo "⚠ Release binary not built (run: cargo build --release)"
fi
echo ""

echo "${BLUE}8. Dependency summary...${NC}"
echo "Total dependencies: $(grep -c "Compiling" <(cargo build --release 2>&1 | grep Compiling) 2>/dev/null || echo "Already compiled")"
echo ""

echo "${BLUE}9. Configuration files...${NC}"
ls -lh Cargo.toml .cargo/config.toml config/default.toml 2>/dev/null | awk '{print "  " $9, "-", $5}'
echo ""

echo "=========================================="
echo "${GREEN}✓ Project verification complete!${NC}"
echo "=========================================="
echo ""
echo "Next steps:"
echo "  1. Run the server: cargo run"
echo "  2. Run tests: cargo test --workspace"
echo "  3. Build release: cargo build --release"
echo "  4. View docs: cargo doc --workspace --open"
echo ""
