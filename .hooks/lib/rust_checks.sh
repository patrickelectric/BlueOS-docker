#!/usr/bin/env bash

# Rust checks of pre-push and CI. They run from core/, the cargo workspace.

rust_changed() {
    local base="${BASE_SHA:-}"
    if [ -z "$base" ]; then
        base=$(git merge-base HEAD origin/master 2>/dev/null) || return 0
    fi
    ! git diff --quiet "$base" HEAD -- 'core/*.rs' 'core/*Cargo.toml' 'core/Cargo.lock' 'core/*api.lock' \
        'core/libs/idl/interfaces'
}

# The generated types are committed, so they must be what the generator writes from the interfaces.
check_generated_idl() {
    echo "Checking the generated IDL code.."
    cargo run --locked --quiet -p blueos-idl-codegen
    git diff --exit-code -- libs/idl/src/generated.rs libs/idl/api.lock
}

# A field or an endpoint that is removed or changed breaks the readers already deployed, so an api.lock may only
# gain lines.
check_api_locks() {
    echo "Checking that no api.lock lost a line.."
    local base="${BASE_SHA:-origin/master}"
    local lock removed
    while IFS= read -r lock; do
        removed=$(comm -23 <(git show "$base:core/$lock" 2>/dev/null | sort) <(sort "$lock"))
        if [ -n "$removed" ]; then
            echo "$lock lost lines of $base, use a new message type or endpoint instead:"
            echo "$removed"
            return 1
        fi
    done < <(git ls-files '*api.lock')
}

# The testing helpers are for tests only, a shipped binary must not have them.
check_testing_feature() {
    echo "Checking that no binary enables the testing feature.."
    if cargo tree --locked --edges normal,features --workspace | grep -q 'blueos-service feature "testing"'; then
        echo "a normal dependency enables the testing feature of blueos-service"
        return 1
    fi
}

# What pre-push runs: the Rust checks, when a Rust file changed.
run_rust_checks() {
    if [ -n "${SKIP_RUST_CHECKS:-}" ] || ! rust_changed; then
        echo "No Rust change, skipping the Rust checks."
        return
    fi
    rust_checks
}

# What CI runs on every change.
rust_checks() {
    (
        set -e
        cd "$CORE_DIR"
        echo "Running cargo fmt.."
        cargo fmt --check
        echo "Running cargo clippy.."
        cargo clippy --locked --all-targets --all-features -- -D warnings
        echo "Running cargo test.."
        cargo test --locked --workspace
        check_generated_idl
        check_api_locks
        check_testing_feature
    )
}
