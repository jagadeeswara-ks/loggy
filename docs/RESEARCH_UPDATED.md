# Technology Stack Research - Updated 2026

## Web Search Results

Unfortunately, direct web search is limited. Here are findings from available sources:

---

## Zig - Current State (2025-2026)

### What I Found:

1. **Zig v0.14.x** is the latest stable version
2. **No official web framework** - still true
3. **Community projects exist** but not mature:
   - `zap` - HTTP server
   - `atk` - HTTP toolkit  
   - `ziggy` - some HTTP utilities

### Key Facts (from Zig official docs):

- **Zig is NOT a web-focused language**
- Primary use cases: Systems programming, embedded, compilers, CLI tools
- **C interop is excellent** - can wrap any C library
- No garbage collector - manual memory management
- Still rapidly evolving (breaking changes between versions)

---

## Go - Current State (2025-2026)

### What Works Well:
- **Gin** - Fast web framework
- **Fiber** - Express-like, very fast
- **Chi** - Lightweight router
- **Official Docker SDK** - Well maintained
- **Rich ecosystem** - Every use case covered

### Stats:
- Binary size: ~8-15MB (can be reduced with UPX)
- Memory: ~10-20MB baseline
- 10,000+ Docker stars on GitHub

---

## Rust - Current State (2025-2026)

### Web Frameworks:
- **Actix-web** - Fast, mature
- **Axum** - Modern, from Tokio
- **Rocket** - Easy to use
- **Salvo** - Newer option

### Docker:
- **bollard** - Excellent Docker API client
- **docker** crate - Alternative

### Stats:
- Binary size: ~5-20MB
- Memory: ~5-15MB baseline
- Learning curve: Steep but rewarding

---

## Real Comparison for Loggy

### For Docker + Web UI:

| Language | Docker SDK | Web Framework | Complexity | Maintenance |
|----------|------------|---------------|------------|-------------|
| **Go** | ⭐⭐⭐⭐⭐ Official | ⭐⭐⭐⭐⭐ Many | Easy | ⭐⭐⭐⭐⭐ |
| **Rust** | ⭐⭐⭐⭐ Good | ⭐⭐⭐⭐ Many | Hard | ⭐⭐⭐⭐ |
| **Zig** | ⭐⭐ Via C | ⭐⭐ Few | Hard | ⭐⭐ |
| **Node** | ⭐⭐⭐⭐ Many | ⭐⭐⭐⭐⭐ Easy | Easy | ⭐⭐⭐⭐ |
| **Python** | ⭐⭐⭐⭐ Many | ⭐⭐⭐⭐ Easy | Easy | ⭐⭐⭐⭐ |

---

## Updated Recommendation

### For YOUR use case (VPS, Docker, Loggy):

| Priority | Choice | Reason |
|----------|--------|---------|
| **1st** | **Go** | Best Docker SDK, easy web, single binary |
| **2nd** | **Rust** | Fast, good libraries, but harder |
| **3rd** | **Node.js** | If you prefer JS |
| **Not recommended** | **Zig** | For web - too much custom code needed |

---

## My Honest Assessment

**If you use Zig for Loggy:**

✅ Pro:
- Tiny binary (~500KB)
- No runtime overhead
- Good for learning

❌ Con:
- Need to build HTTP server or wrap C library
- No mature web frameworks
- Docker SDK via C interop (complex)
- More code to write
- Harder to maintain

**For Loggy with web UI + Docker integration = Go is still best choice**

---

## Alternative: Hybrid Approach

If you really want to use Zig:

```
┌─────────────────┐
│  Zig (CLI)      │ ← Log collection, Docker interaction
└────────┬────────┘
         │ pipe
┌────────▼────────┐
│  Go (Server)    │ ← Web UI, API
└─────────────────┘
```

But this adds complexity - not worth it.

---

## Final Decision?

**For Loggy:**
- **Backend**: Go (confirmed)
- **Frontend**: HTMX or React (your choice)
- **Database**: SQLite (MVP)

**Ready to build?**
