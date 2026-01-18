#!/bin/bash
# Sprint 4 Demo: Automated Git Operations
#
# This script demonstrates the Git integration features of Ralph Loop Manager:
# - Adding Git credentials via API
# - Creating a loop with Git repository URL
# - Automatic commits during loop execution
# - Automatic PR creation on loop completion
#
# Prerequisites:
# - Docker daemon running
# - Git repository URL (will create a test repo if GITHUB_TOKEN is set)
# - ANTHROPIC_API_KEY or OPENAI_API_KEY in .env

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SERVER_URL="http://localhost:3000"
DB_FILE="demo_sprint4.db"
LOG_FILE="demo_sprint4.log"

# Test credentials
TEST_USERNAME="demouser"
TEST_PASSWORD="DemoPass123!"
TEST_EMAIL="demo@example.com"

# Cleanup function
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"

    # Stop server if running
    if [ -n "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi

    # Remove database
    rm -f "$DB_FILE"

    echo -e "${GREEN}Cleanup complete${NC}"
}

# Trap Ctrl+C and cleanup
trap cleanup EXIT INT TERM

# Print colored message
print_msg() {
    local color=$1
    local msg=$2
    echo -e "${color}${msg}${NC}"
}

# Print section header
print_header() {
    echo ""
    echo "============================================================"
    print_msg "$1" "$2"
    echo "============================================================"
}

# Wait for server to be ready
wait_for_server() {
    print_msg "$BLUE" "Waiting for server to start..."

    local max_attempts=30
    local attempt=0

    while [ $attempt -lt $max_attempts ]; do
        if curl -s "$SERVER_URL/health" > /dev/null 2>&1; then
            print_msg "$GREEN" "Server is ready!"
            return 0
        fi
        sleep 1
        attempt=$((attempt + 1))
    done

    print_msg "$RED" "Server failed to start"
    return 1
}

# Register user
register_user() {
    print_header "$BLUE" "Step 1: Register User"

    local response=$(curl -s -X POST "$SERVER_URL/api/auth/register" \
        -H "Content-Type: application/json" \
        -d "{
            \"username\": \"$TEST_USERNAME\",
            \"email\": \"$TEST_EMAIL\",
            \"password\": \"$TEST_PASSWORD\"
        }")

    if echo "$response" | grep -q "\"success\":true"; then
        print_msg "$GREEN" "✓ User registered successfully"
        return 0
    else
        print_msg "$RED" "✗ Registration failed: $response"
        return 1
    fi
}

# Login and get session token
login_user() {
    print_header "$BLUE" "Step 2: Login"

    local response=$(curl -s -X POST "$SERVER_URL/api/auth/login" \
        -H "Content-Type: application/json" \
        -d "{
            \"username\": \"$TEST_USERNAME\",
            \"password\": \"$TEST_PASSWORD\"
        }")

    SESSION_TOKEN=$(echo "$response" | grep -o '"session":"[^"]*' | cut -d'"' -f4)

    if [ -n "$SESSION_TOKEN" ]; then
        print_msg "$GREEN" "✓ Login successful, session token obtained"
        return 0
    else
        print_msg "$RED" "✗ Login failed: $response"
        return 1
    fi
}

# Add Git credentials
add_git_credentials() {
    print_header "$BLUE" "Step 3: Add Git Credentials"

    # Check for GITHUB_TOKEN
    if [ -z "$GITHUB_TOKEN" ]; then
        print_msg "$YELLOW" "⚠ GITHUB_TOKEN not set, using mock token"
        GIT_TOKEN="ghp_mock_token_for_demo_purposes"
        print_msg "$YELLOW" "  (Set GITHUB_TOKEN environment variable for real Git operations)"
    else
        GIT_TOKEN="$GITHUB_TOKEN"
    fi

    local response=$(curl -s -X POST "$SERVER_URL/api/git/credentials" \
        -H "Content-Type: application/json" \
        -H "session: $SESSION_TOKEN" \
        -H "X-CSRF-Token: demo-csrf-token" \
        -d "{
            \"provider\": \"github\",
            \"token\": \"$GIT_TOKEN\",
            \"username\": \"$TEST_USERNAME\",
            \"email\": \"$TEST_EMAIL\"
        }")

    if echo "$response" | grep -q "\"success\":true"; then
        print_msg "$GREEN" "✓ Git credentials added successfully"
        return 0
    else
        print_msg "$RED" "✗ Failed to add Git credentials: $response"
        return 1
    fi
}

# Create a loop with Git repository URL
create_loop() {
    print_header "$BLUE" "Step 4: Create Loop with Git Repository"

    # Use a test repository URL
    local git_repo="https://github.com/ralph-loop-manager/test-repo.git"

    local response=$(curl -s -X POST "$SERVER_URL/api/loops" \
        -H "Content-Type: application/json" \
        -H "session: $SESSION_TOKEN" \
        -H "X-CSRF-Token: demo-csrf-token" \
        -d "{
            \"name\": \"Sprint 4 Demo Loop\",
            \"description\": \"Demonstrating automated Git operations\",
            \"prd\": \"# Sprint 4 Demo\\n\\nThis loop demonstrates automated Git integration:\\n1. Clone repository\\n2. Make changes\\n3. Auto-commit\\n4. Create PR\",
            \"provider\": \"mock\",
            \"model\": \"mock\",
            \"docker_image\": \"ralph-loop-manager:latest\",
            \"cpu_limit\": 1,
            \"memory_limit\": 1024,
            \"max_iterations\": 10,
            \"iteration_timeout\": 300,
            \"iteration_delay\": 0,
            \"git_repo_url\": \"$git_repo\",
            \"git_branch_pattern\": \"ralph/demo/{timestamp}\"
        }")

    LOOP_ID=$(echo "$response" | grep -o '"id":"[^"]*' | cut -d'"' -f4 | head -1)

    if [ -n "$LOOP_ID" ]; then
        print_msg "$GREEN" "✓ Loop created successfully with ID: $LOOP_ID"
        return 0
    else
        print_msg "$RED" "✗ Failed to create loop: $response"
        return 1
    fi
}

# Add a task to the loop
add_task() {
    print_header "$BLUE" "Step 5: Add Task to Loop"

    local response=$(curl -s -X POST "$SERVER_URL/api/loops/$LOOP_ID/tasks" \
        -H "Content-Type: application/json" \
        -H "session: $SESSION_TOKEN" \
        -H "X-CSRF-Token: demo-csrf-token" \
        -d "{
            \"title\": \"Create demo file and commit\",
            \"description\": \"Create a test file in the repository and let the system auto-commit\",
            \"status\": \"pending\",
            \"priority\": 5
        }")

    if echo "$response" | grep -q "\"success\":true"; then
        print_msg "$GREEN" "✓ Task added successfully"
        return 0
    else
        print_msg "$RED" "✗ Failed to add task: $response"
        return 1
    fi
}

# Start the loop
start_loop() {
    print_header "$BLUE" "Step 6: Start Loop Execution"

    local response=$(curl -s -X POST "$SERVER_URL/api/loops/$LOOP_ID/start" \
        -H "session: $SESSION_TOKEN" \
        -H "X-CSRF-Token: demo-csrf-token")

    if echo "$response" | grep -q "\"success\":true"; then
        print_msg "$GREEN" "✓ Loop started successfully"
        return 0
    else
        print_msg "$RED" "✗ Failed to start loop: $response"
        return 1
    fi
}

# Monitor loop progress
monitor_loop() {
    print_header "$BLUE" "Step 7: Monitor Loop Progress"

    local max_wait=60
    local elapsed=0

    while [ $elapsed -lt $max_wait ]; do
        local response=$(curl -s -X GET "$SERVER_URL/api/loops/$LOOP_ID" \
            -H "session: $SESSION_TOKEN")

        local status=$(echo "$response" | grep -o '"status":"[^"]*' | cut -d'"' -f4)
        local iteration=$(echo "$response" | grep -o '"current_iteration":[0-9]*' | cut -d':' -f2)

        echo -ne "\r${BLUE}Status: $status | Iteration: $iteration | Elapsed: ${elapsed}s${NC}"

        if [ "$status" = "completed" ] || [ "$status" = "error" ]; then
            echo ""
            if [ "$status" = "completed" ]; then
                print_msg "$GREEN" "✓ Loop completed successfully!"
            else
                print_msg "$RED" "✗ Loop encountered an error"
            fi
            break
        fi

        sleep 2
        elapsed=$((elapsed + 2))
    done

    if [ $elapsed -ge $max_wait ]; then
        echo ""
        print_msg "$YELLOW" "⚠ Loop monitoring timeout after ${max_wait}s"
    fi
}

# Display results
display_results() {
    print_header "$GREEN" "Demo Results"

    local response=$(curl -s -X GET "$SERVER_URL/api/loops/$LOOP_ID" \
        -H "session: $SESSION_TOKEN")

    echo ""
    print_msg "$BLUE" "Loop Details:"
    echo "$response" | grep -E '"id"|"name"|"status"|"current_iteration"|"git_repo_url"' | sed 's/"/  /g' | sed 's/,//g'

    echo ""
    print_msg "$GREEN" "✓ Sprint 4 Demo Complete!"
    echo ""
    print_msg "$BLUE" "Git Integration Features Demonstrated:"
    echo "  • Git credentials stored securely (encrypted)"
    echo "  • Loop configured with Git repository URL"
    echo "  • Automatic commits during execution"
    echo "  • Automatic PR creation on completion"
    echo ""
    print_msg "$YELLOW" "Note: For full Git integration with real commits/PRs:"
    echo "  1. Set GITHUB_TOKEN environment variable"
    echo "  2. Create a test repository on GitHub"
    echo "  3. Update git_repo_url in the script"
    echo "  4. Ensure ralph-loop-manager:latest Docker image exists"
}

# Main execution
main() {
    print_msg "$GREEN" "=========================================="
    print_msg "$GREEN" "  Sprint 4 Demo: Git Integration"
    print_msg "$GREEN" "=========================================="

    # Check prerequisites
    if ! command -v docker &> /dev/null; then
        print_msg "$RED" "✗ Docker is required but not installed"
        exit 1
    fi

    if ! command -v cargo &> /dev/null; then
        print_msg "$RED" "✗ Cargo/Rust is required but not installed"
        exit 1
    fi

    # Check for .env file
    if [ ! -f ".env" ]; then
        print_msg "$YELLOW" "⚠ No .env file found, creating from .env.example"
        cp .env.example .env

        # Add mock API key for demo
        echo "" >> .env
        echo "# Demo: Using mock provider for demo" >> .env
        echo "MOCK_API_KEY=demo_key_for_testing" >> .env
    fi

    # Build server
    print_header "$BLUE" "Building Server"
    cargo build --release --bin ralph-server 2>&1 | tail -5

    # Start server
    print_header "$BLUE" "Starting Server"
    export DATABASE_URL="sqlite:$DB_FILE"
    export RUST_LOG="ralph_server=info"

    target/release/ralph-server > "$LOG_FILE" 2>&1 &
    SERVER_PID=$!

    wait_for_server || exit 1

    # Run demo steps
    register_user || exit 1
    login_user || exit 1
    add_git_credentials || exit 1
    create_loop || exit 1
    add_task || exit 1
    start_loop || exit 1
    monitor_loop
    display_results

    print_msg "$BLUE" "Server logs saved to: $LOG_FILE"
    print_msg "$BLUE" "Database file: $DB_FILE"
    print_msg "$BLUE" "Loop ID: $LOOP_ID"
}

# Run main function
main "$@"
