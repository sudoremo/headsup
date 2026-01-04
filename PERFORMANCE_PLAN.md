# Headsup Performance Optimization Plan

## Problem Summary

The `headsup check` command times out (60s) frequently when checking 12 subjects sequentially. Each subject:
- Spawns a new `claude` CLI process (~1-2s overhead)
- Performs web searches via Claude's WebSearch tool (30-50+ seconds)
- Processes results and updates state

Current bottlenecks:
1. **Sequential execution** - 12 subjects × 60s timeout = 12 minutes maximum
2. **Broad search terms** - Generic terms like "apple", "event", "keynote" cause extensive searches
3. **Redundant search_terms** - Claude could infer better queries from descriptions

## Proposed Optimizations

### 1. Make search_terms Optional ✅ HIGH VALUE, LOW EFFORT

**Rationale:** Claude receives rich context (subject name, description, notes, type) and can intelligently formulate search queries. Explicit search_terms are often too broad or redundant.

**Changes Required:**

#### A. Config Types (`src/config/types.rs`)

**Line 133:** Keep as `Vec<String>` (no type change needed)
```rust
pub search_terms: Vec<String>,  // Keep as-is, just allow empty
```

**Lines 194-196:** Remove validation requirement
```rust
// DELETE these lines:
if self.search_terms.is_empty() {
    return Err("At least one search term is required".to_string());
}
```

#### B. Prompt Generation (`src/claude/prompt.rs`)

**Lines 7, 70, 131:** Handle empty search_terms conditionally

For **release prompts** (line 7):
```rust
let search_terms_section = if subject.search_terms.is_empty() {
    String::new()
} else {
    format!("SEARCH TERMS: {}\n", subject.search_terms.join(", "))
};

// Then in the format! macro, replace line 30:
{search_terms_section}  // Instead of hard-coded SEARCH TERMS line
```

For **question prompts** (line 70):
```rust
let search_terms_section = if subject.search_terms.is_empty() {
    String::new()
} else {
    format!("SEARCH TERMS: {}\n", subject.search_terms.join(", "))
};

// Update format! macro at line 92 similarly
```

For **recurring prompts** (line 131):
```rust
let search_terms_section = if subject.search_terms.is_empty() {
    String::new()
} else {
    format!("SEARCH TERMS: {}\n", subject.search_terms.join(", "))
};

// Update format! macro at line 159 similarly
```

**Important:** When search_terms are omitted, the prompt should instruct Claude to determine appropriate queries:
- For releases: Add "Search for recent news about {name} ({category})"
- For questions: Add "Research: {question}"
- For recurring: Add "Search for next occurrence of {event_name}"

#### C. CLI Subject Management (`src/cli/subjects.rs`)

**Lines 219-228:** Make search_terms optional during add
```rust
// Prompt with note that it's optional
let search_terms_input = ui::prompt_text("Search terms (comma-separated, or press Enter to let Claude decide):")?;
let search_terms: Vec<String> = if search_terms_input.trim().is_empty() {
    Vec::new()  // Allow empty
} else {
    search_terms_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
};

// Remove the validation check (lines 226-228)
```

**Lines 314-320:** Make search_terms optional during edit
```rust
let current_terms = if subject.search_terms.is_empty() {
    String::new()
} else {
    subject.search_terms.join(", ")
};
let new_terms = ui::prompt_text_with_default(
    "Search terms (comma-separated, or leave empty):",
    &current_terms
)?;
subject.search_terms = if new_terms.trim().is_empty() {
    Vec::new()
} else {
    new_terms
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
};
```

**Backwards Compatibility:** Existing configs with search_terms will continue to work.

---

### 2. Parallel Execution ✅ HIGH VALUE, MEDIUM EFFORT

**Rationale:** Claude API calls are independent and take 30-60s each. Running in parallel will reduce total runtime from 12 minutes to ~60s (limited by slowest subject).

**Implementation Strategy:**

Use `tokio::join_all` to spawn all checks concurrently, collect results, then process state updates sequentially to maintain consistency.

**Changes Required:**

#### `src/cli/check.rs`

**Replace sequential loop (lines 59-90) with parallel execution:**

```rust
// After line 58, replace the entire for loop with:

// Spawn all checks in parallel
let mut check_futures = Vec::new();
for subject in subjects_to_check {
    // Clone config for move into async task
    let config_clone = config.clone();
    let subject_clone = subject.clone();
    let state_for_check = state.subjects.get(&subject.id).cloned();

    // Spawn async task
    let future = tokio::spawn(async move {
        let result = check_single_subject_parallel(
            &config_clone,
            &subject_clone,
            state_for_check.as_ref(),
        ).await;
        (subject_clone, result)
    });

    check_futures.push(future);
    search_count += 1;

    // Check limits
    if search_count >= config.claude.max_searches_per_run {
        break;
    }
}

// Wait for all checks to complete (with timeout)
let timeout_remaining = total_timeout
    .map(|t| t.saturating_sub(start.elapsed()))
    .unwrap_or(Duration::from_secs(3600));

let check_results = match tokio::time::timeout(
    timeout_remaining,
    futures::future::join_all(check_futures)
).await {
    Ok(results) => results,
    Err(_) => {
        ui::print_warning("Total run timeout exceeded during parallel execution");
        Vec::new()
    }
};

// Process results sequentially to update state
let mut results: Vec<CheckResult> = Vec::new();
for join_result in check_results {
    match join_result {
        Ok((subject, check_result)) => {
            match check_result {
                Ok((claude_response, should_notify)) => {
                    // Update state (existing logic from process_*_response functions)
                    process_result_and_update_state(
                        &config,
                        &subject,
                        claude_response,
                        should_notify,
                        &mut state,
                        dry_run,
                        no_notify,
                        &mut results,
                    );
                }
                Err(e) => {
                    // Handle error (existing logic from lines 181-203)
                    handle_check_error(&config, &subject, e, &mut state, dry_run, &mut results);
                }
            }
        }
        Err(e) => {
            ui::print_error(&format!("Task join error: {}", e));
        }
    }
}
```

**New function:** `check_single_subject_parallel` (return result instead of updating state directly)

```rust
async fn check_single_subject_parallel(
    config: &Config,
    subject: &Subject,
    state: Option<&SubjectState>,
) -> Result<(ClaudeResponse, bool)> {
    // Call Claude
    let response = claude::check_subject(&config.claude, subject, state).await?;

    // Determine if should notify based on response
    let should_notify = match &response {
        ClaudeResponse::Release(r) => r.should_notify,
        ClaudeResponse::Question(r) => r.should_notify,
        ClaudeResponse::Recurring(r) => r.should_notify,
        ClaudeResponse::SubjectIdentification(_) => false,
    };

    Ok((response, should_notify))
}
```

**Extract helper functions:**
- `process_result_and_update_state()` - Combines process_*_response and notification logic
- `handle_check_error()` - Combines error handling logic

**Dependencies:** Add to `Cargo.toml`:
```toml
futures = "0.3"
```

**Performance Expectations:**
- Current: 12 subjects × 60s = ~12 minutes maximum (sequential)
- Optimized: ~60s total (parallelized, limited by slowest subject)
- **10-12x speedup** for typical runs

**Trade-offs:**
- More complex error handling
- Results may complete in non-deterministic order
- Higher instantaneous load (12 Claude processes at once)
- Still maintains state consistency via sequential updates

---

### 3. Direct API Calls ❌ SKIP FOR NOW

**Recommendation:** Do NOT implement direct API calls.

**Rationale:**
1. **Low ROI:** Process overhead is ~1-2s per call × 12 subjects = 12-24s
   - Web searches take 30-50s per subject (the real bottleneck)
   - Parallel execution provides **10-12x speedup** vs 20% from eliminating process overhead

2. **High Complexity:**
   - Need to add HTTP client dependency (reqwest)
   - Implement Anthropic Messages API integration
   - Handle authentication (OAuth/API keys)
   - **Most critically:** Implement WebSearch tool support
     - Claude CLI provides this seamlessly via `--allowedTools WebSearch`
     - Direct API would require implementing tool use protocol
     - Much more complex than simple API calls

3. **Maintenance Burden:**
   - CLI interface is simpler and maintained by Anthropic
   - Direct API requires keeping up with API changes
   - Tool use implementation is non-trivial

**Future Consideration:** If CLI becomes a bottleneck after other optimizations, revisit this.

---

## Implementation Order

1. **search_terms optional** (30 minutes)
   - Simple changes across 4 files
   - Test with empty search_terms
   - Update config file

2. **Parallel execution** (2-3 hours)
   - Refactor check.rs for concurrent execution
   - Add futures dependency
   - Extensive testing for race conditions
   - Verify state consistency

3. **Test and validate** (1 hour)
   - Run `headsup check` with optimizations
   - Verify all subjects checked correctly
   - Confirm timeout issues resolved
   - Monitor for any state corruption

## Critical Files

- `src/config/types.rs` - Subject struct, validation
- `src/claude/prompt.rs` - Prompt generation
- `src/cli/subjects.rs` - Interactive subject management
- `src/cli/check.rs` - Main check loop (parallel execution)
- `Cargo.toml` - Dependencies

## Testing Plan

1. **Unit tests:** Verify prompt generation with/without search_terms
2. **Integration test:** Run `headsup check` on a few subjects
3. **Full test:** Run on all 12 subjects, verify timeout doesn't occur
4. **State consistency:** Verify state file correctly updated after parallel execution
5. **Error handling:** Test with intentional failures to verify consecutive_failures tracking

## Rollback Strategy

If issues arise:
1. **search_terms:** Easy - just add back validation, require at least one term
2. **Parallel execution:** Revert check.rs to sequential loop (git revert)
3. Config changes are backward compatible (empty search_terms just omit the field in prompt)
