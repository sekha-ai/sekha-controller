# Sekha Controller - Development TODO

## ✅ COMPLETED (v0.1.1)

### Core Functionality
- ✅ MCP memory_export endpoint (full conversation export)
- ✅ MCP memory_stats endpoint (global and folder-scoped stats)
- ✅ Repository methods: `get_message_list()`, `get_stats()`
- ✅ FTS5 full-text search with automatic indexing triggers
- ✅ Database update triggers for `updated_at` timestamps
- ✅ WAL mode for concurrent database access
- ✅ Migration schema validation tests
- ✅ 44 integration tests passing
- ✅ UUID/BLOB handling fixed for SQLite
- ✅ Modular test structure (`tests/integration/` modules)
- ✅ Tarpaulin configured to exclude patches
- ✅ Basic unit tests (route, queue, services construction)

---

## 🎯 CURRENT FOCUS: Coverage → 80%+ (Option B: Mockall Path)

**Current Baseline:** 45-47% coverage (956/2121 lines)  
**Target Goal:** 80% (1,697 lines) = **+741 lines needed**  
**Stretch Goal:** 90% (1,909 lines) = **+953 lines needed**

---

## Phase 1: Add Mockall Framework ⏱️ 30 min

**Status:** 🔴 Not Started

### Tasks:
- [ ] Add `mockall = "0.13"` to Cargo.toml dev-dependencies
- [ ] Make `ConversationRepository` trait mockable with `#[cfg_attr(test, mockall::automock)]`
- [ ] Make `LlmBridgeClient` mockable
- [ ] Make `ChromaClient` mockable
- [ ] Verify mock generation works: `cargo test --test unit`

**Gain:** Infrastructure for all remaining tests  
**Blockers:** None

---

## Phase 2: Mock-Based Unit Tests ⏱️ 4-6 hours

**Status:** 🔴 Not Started

### Priority 1: Orchestrator Layer (+150 lines)

**Files to test:**
- [ ] `src/orchestrator/importance_engine.rs` (26 lines)
  - Test `calculate_score()` with mocked repo + LLM
  - Test edge cases (empty messages, LLM errors)
  
- [ ] `src/orchestrator/pruning_engine.rs` (28 lines)
  - Test `generate_suggestions()` with mocked repo
  - Test various importance thresholds

- [ ] `src/orchestrator/label_intelligence.rs` (17 lines uncovered)
  - Test `suggest_labels()` with mocked LLM
  - Test `auto_label()` workflow

- [ ] `src/orchestrator/context_assembly.rs` (24 lines uncovered)
  - Test context building with mocked repo

### Priority 2: Service Layer (+138 lines)

**Files to test:**
- [ ] `src/services/chroma_client.rs` (81 lines)
  - Mock HTTP responses with `mockito`
  - Test `store_embedding()`, `search_similar()`, `delete()`
  
- [ ] `src/services/embedding_service.rs` (57 lines)
  - Mock Ollama HTTP calls
  - Test embedding generation, error handling

### Priority 3: API Layer (+155 lines)

**Files to test:**
- [ ] `src/api/routes.rs` (155 lines uncovered)
  - Test error paths (repo errors → 500 responses)
  - Test validation errors (invalid UUID → 400)
  - Test authentication failures

**Estimated Gain:** ~443 lines (+21% coverage)

---

## Phase 3: Enhanced Integration Tests ⏱️ 2-3 hours

**Status:** 🟡 Partially Done (44 tests exist)

### Add These Scenarios:
- [ ] Large dataset (100+ conversations) performance test
- [ ] Concurrent writes stress test (already have basic concurrency test)
- [ ] Error recovery tests (database locked, out of disk, etc.)
- [ ] MCP auth edge cases (expired keys, wrong format)
- [ ] REST API comprehensive error paths

**Estimated Gain:** ~150 lines (+7% coverage)

---

## Phase 4: File Watcher Tests ⏱️ 2 hours

**Status:** 🔴 Disabled (commented out in integration tests)

**Current:** 8/336 lines (2.4%)

### Tasks:
- [ ] Create temp directory tests
- [ ] Test ChatGPT import parsing
- [ ] Test Claude import parsing
- [ ] Test error handling (malformed JSON, missing files)
- [ ] Re-enable in `tests/integration/mod.rs`

**Estimated Gain:** ~200 lines (+9% coverage)

---

## Coverage Roadmap

| Phase | Effort | Lines Gained | New Coverage | Status |
|-------|--------|--------------|--------------|--------|
| **Baseline** | - | - | 47% | ✅ |
| Phase 1: Mockall Setup | 30 min | 0 | 47% | 🔴 |
| Phase 2: Mock Tests | 4-6 hrs | +443 | **68%** | 🔴 |
| Phase 3: Integration | 2-3 hrs | +150 | **75%** | 🔴 |
| Phase 4: File Watcher | 2 hrs | +200 | **84%** | 🔴 |
| **🎯 TARGET 80%** | **~10 hrs** | **+593** | **80%** ✅ | 🔴 |
| **🌟 STRETCH 90%** | **+3 hrs** | **+200** | **90%** | 🟡 |

---

## 📋 Backlog (Post-80% Coverage)

### Infrastructure
- [ ] CI/CD coverage reporting (upload to Codecov/Coveralls)
- [ ] Benchmark tests for FTS performance
- [ ] Docker compose for test environment (Chroma + Ollama)
- [ ] Upgrade to SeaORM 2.0.0 stable (when released)

### Documentation
- [ ] Update `docs/architecture/mcp-protocol.md` with export/stats
- [ ] Update `docs/api/mcp-reference.md` with new endpoints
- [ ] Add coverage badge to README
- [ ] Update CHANGELOG.md for v0.1.1

---

## 🚀 Immediate Next Steps

1. ✅ Fix route_test.rs compilation error
2. ✅ Run tests and confirm 47% baseline
3. 🔴 **Add mockall to Cargo.toml** (5 min)
4. 🔴 **Make traits mockable** (15 min)
5. 🔴 **Write first mocked test** (importance_engine) (1 hour)
6. 🔴 **Verify coverage jumps to 50%+**
7. 🔴 **Continue with remaining mock tests**

---

## Notes

- ✅ SeaORM patch excluded from coverage
- ✅ 44 integration tests passing
- ✅ FTS5, triggers, WAL operational
- 🔴 Mockall framework needed for orchestrator/service tests
- 🎯 Realistic path to 80% within 10 hours
- 🌟 90% achievable with file_watcher + edge cases
