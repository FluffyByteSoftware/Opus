<!--
File:       Opus/Documentation/LLM/STATUS.md
Component:  Documentation
Author:     Jacob Chacko
-->

# Opus -- Status

## Where things stand

Conductor is four crates.  `conductor-tools` (lib) holds DiskMan, Scribe, Constellations, Fingerprinter,
Security, Archivist, the notices, the clock, the thread list and the services list.  `conductor-monitor`
(lib) looks at the process and every process on the machine once a second.  `conductor-wgui` (lib) is the
web admin at `http://127.0.0.1:9996/Opus`, and the only way to shut the server down.  `conductor-launcher`
(bin) starts all of it and waits on the web admin.  Ensemble hasn't been started.

**Last built and tested on Linux (Nobara 44), 2026-09-28**, before Security: `cargo build` clean with no
warnings, `cargo test` passing.  Fingerprinter went in after that and was merged.  **Security is written and
not built yet**; this session's branch, `infamous-saganism`, is waiting on Jacob's build, tests and benchmark.
The Windows code has never been built.

## Last session -- 2026-09-29

Security, the password hasher, on the branch `infamous-saganism`.  Modelled on Stratum's `security.rs`, with
the emphasis this time on spending less CPU per hash and more RAM where that buys the same protection.

What we did:

- **`security.rs`**: Argon2id through the `argon2` crate (0.6, default features off; our second crate).
  `hash_password()`, `verify_password()` and `verify_no_account()` each hand back a `Pending`, like
  Archivist and DiskMan, so nothing waits on a hash.  `check_password_rules()` (Jacob's rules: 8 to 128
  printable ASCII, a digit, a capital, a symbol) and `pad_login_time()` (150 ms floor) came over from
  Stratum as they were.
- **One worker thread, `security`, one arena.**  The worker allots 64 MiB of Argon2 blocks once, when it
  starts, and every hash runs in it through `hash_password_into_with_memory()`, so the OS isn't asked for a
  fresh 64 MiB (and 16,384 page faults) on every hash.  A stored line that wants more memory than the arena
  holds gets a one-off allocation instead of no answer.
- **One pass, not two.**  Argon2's CPU time is memory times passes; the memory is what an attacker's
  graphics card is short of.  Same 64 MiB, half the CPU.  To make a hash harder later, raise the memory.
- **The huge page hint on Linux**, `madvise(MADV_HUGEPAGE)` on the arena before its pages are touched, in
  `security/linux.rs` behind the usual per-OS `advise_huge_pages()`.  Windows and macOS say they have no
  hint.  The Services tab note says whether the kernel took it.
- **Fingerprinter grew `random_bytes()`**, the raw OS random source, for the salts.  The crate's own random
  source stays out of the build.
- **Wired in**: `services::SECURITY` on the expected list (thread `security`, checks in every second), the
  launcher starts it after Fingerprinter and stops it before Archivist.
- Twelve tests, most at the cheapest Argon2 setting in an arena of their own, one at the real cost, one
  checking our PHC line is byte-for-byte what the crate's own hasher writes.  And the benchmark,
  `cargo test -p conductor-tools --release argon2_cost -- --ignored --nocapture`: five memory settings at one and two
  passes, three ways each (fresh memory, the arena, the arena with huge pages).
- The docs caught up on Fingerprinter, which the last hand-off predates: PROJECT_OPUS.md, the tools design
  doc and the web admin's services table.

What fought back:

- Nothing could be built or run here, so the crate's API was checked against its source (argon2 0.6.0,
  password-hash 0.6.0, phc 0.6.1) rather than a compiler.  Stratum's file used the 0.6 API already, which
  helped.  The first build is the real check.

What Jacob decided:

- The branch is `infamous-saganism`.
- Security's job is the theory, not a bulletproof server: get the trade-offs right and written down, measure
  them, and don't gold-plate.

## What's waiting

- **Build and test Security**, from `Conductor/dev`: `cargo build`, then `cargo test -p conductor-tools`,
  then the benchmark.  The memory setting (64 MiB) may go up once the benchmark says what one pass costs.
- Whether Nobara's kernel takes the huge page hint: `cat /sys/kernel/mm/transparent_hugepage/enabled`
  (`[madvise]` or `[always]` means yes), and the benchmark's "huge" column against "arena".
- Accounts: making, checking and logging in.  Security and Fingerprinter are ready for it; the login itself
  waits on networking.
- Archivist retrying on its own every 5 seconds while disconnected, so the page's lock lifts when Postgres
  comes back.  Asked, not answered.
- DiskMan: seeing hand edits to a file it already holds.  In TODO.
- The Windows build, whenever getting to that machine is less of a hassle.  The probe, the process list,
  the threads route, DiskMan's rename, Fingerprinter's `BCryptGenRandom` and Security's "no hint" file are
  all untried there.
- The Debug switch in `conductor_globals.cfg`, and moving the routine log lines to Debug.  It matters more
  now: every Warn is a notice, so a Warn that isn't really wrong is one more thing to ACK.
- Catching Ctrl-C, now that it can lose what DiskMan holds.
- `\dt` in psql to confirm `archivist_migrations` exists, and that `0001_uuid_on_every_table.sql` ran.
- Picking Ensemble's engine.
