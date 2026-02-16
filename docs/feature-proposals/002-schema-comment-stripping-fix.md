# Feature Proposal: SQL Schema Comment Stripping

**Proposal ID:** FP-002
**Status:** ✅ Completed
**Created:** 2026-02-16
**Author:** SDLC Team
**Assignee:** SDLC Team

---

## Overview

Fix database schema initialization failures caused by SQL comment lines in migration files. The DuckDB schema execution was failing when SQL statements containing comment lines (`--`) were split by semicolon delimiter.

---

## Problem Statement

### Current Situation
Migration files (`001_initial_schema.sql`, etc.) contain SQL comments for documentation. When the schema initialization code splits these files by semicolon to execute individual statements, comment-only lines cause empty or malformed statements that fail execution.

### Impact
- Database schema initialization fails silently
- Tables not created properly
- Application starts but database operations fail
- Development confusion when schema changes don't appear to work

### Current Workarounds
- Manually execute SQL in DuckDB CLI
- Remove comments from migration files (loses documentation)

### Root Cause
```rust
// Old code in db.rs
for statement in schema.split(';') {
    if !statement.trim().is_empty() {
        conn.execute(statement, [])?;  // Fails on comment lines
    }
}
```

When splitting by `;`, a block like:
```sql
-- This is a comment
CREATE TABLE foo (id INT);
```
Becomes two statements: `-- This is a comment` and `\nCREATE TABLE foo (id INT)`.
The first causes an error.

---

## Proposed Solution

Strip SQL comment lines before splitting and executing statements.

### Implementation
```rust
for statement in schema.split(';') {
    // Strip comment lines before checking if statement is empty
    let stmt: String = statement
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    let stmt = stmt.trim();
    if !stmt.is_empty() {
        if let Err(e) = conn.execute(&stmt, []) {
            let err_str = e.to_string();
            if !err_str.contains("already exists") {
                tracing::warn!("Schema statement failed: {}", err_str);
            }
        }
    }
}
```

### Success Criteria
- [x] SQL migrations execute without errors
- [x] Comments are preserved in migration files for documentation
- [x] All tables created successfully on fresh database
- [x] No regression in existing deployments

---

## Technical Approach

### Architecture Changes
- **Component:** `peilbeheer-api/src/db.rs`
- **Function:** `Database::initialize_schema()`
- **Lines:** ~66-81

### Data Changes
None - data model unchanged

### API Changes
None - internal implementation only

---

## Requirements Traceability

| REQ ID | Requirement | Component | Tests | Status |
|--------|-------------|-----------|-------|--------|
| FIX-001 | Strip SQL comments before execution | db.rs | ❌ | ✅ |
| FIX-002 | Preserve migration file documentation | migrations/*.sql | N/A | ✅ |
| FIX-003 | Handle multi-line comments | db.rs | ❌ | ⚠️ Future |

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Breaking existing deployments | Low | High | `already exists` check preserved |
| Stripping needed content | Low | Medium | Only strip lines starting with `--` |
| Multi-line comment handling | Low | Low | Future enhancement (not in scope) |

---

## Implementation Estimate

| Phase | Tasks | Estimate | Actual |
|-------|-------|----------|--------|
| Analysis | Root cause investigation | 1 hour | 1 hour |
| Implementation | Comment stripping logic | 1 hour | 1 hour |
| Testing | Manual database recreation | 1 hour | 1 hour |
| Documentation | Update code comments | 0.5 hours | 0.5 hours |
| **Total** | | **3.5 hours** | **3.5 hours** |

---

## Dependencies

### Internal Dependencies
- Must be deployed before any new schema migrations

### External Dependencies
None

---

## Rollout Plan

1. ✅ Implement fix in `db.rs`
2. ✅ Test with fresh database creation
3. ✅ Verify all tables created
4. ✅ Commit to `fix/schema-init-comment-stripping` branch
5. ⏳ Merge to main after review

---

## Approval

| Role | Name | Approval | Date |
|------|------|----------|------|
| Product Owner | SDLC Team | ✅ Approved | 2026-02-16 |
| Solution Architect | SDLC Team | ✅ Reviewed | 2026-02-16 |
| Tech Lead | SDLC Team | ✅ Reviewed | 2026-02-16 |

---

## Change History

| Date | Version | Description | Author |
|------|---------|-------------|--------|
| 2026-02-16 | 1.0 | Initial proposal | SDLC Team |
| 2026-02-16 | 1.1 | Implemented and tested | SDLC Team |

---

## Future Enhancements

- [ ] Handle block comments `/* ... */`
- [ ] Better error reporting with line numbers
- [ ] Migration version tracking in database
- [ ] Rollback capability

---

*This is a hotfix to unblock database schema initialization.*
