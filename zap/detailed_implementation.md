You are implementing `zap_orchestrator` in `/home/zakke/ProgStuff/egui/zap`.

Your job is to make the architecture and implementation decisions needed to integrate the orchestrator with the existing zap tooling. Read the actual code before editing. The supplied design description may not perfectly reflect the current implementation; when the description and code disagree, the code wins. Identify the discrepancy and design around the actual code.

First read `/home/zakke/ProgStuff/egui/zap/gemini_orchestrator_plan.md` in full. It specifies the orchestration loop, history, payload packaging, Gemini calls, and rate limiting. Then inspect the wire crate, both clients' socket integration, zapgui's listener and request router, chain review/outcome handling, the Cargo manifests, `local_cargo.sh`, and the existing egui application setup.

Do not merely produce another plan. Implement as much as can be implemented safely without guessing the unresolved product decisions at the end of this prompt.

### 1. Establish the correct project structure

The supplied source dump suggests that the root directory coordinates separate Cargo projects rather than being a conventional workspace with a root `Cargo.toml`. Verify that against the actual filesystem.

Create `zap_orchestrator` as a sibling package following the repository's existing conventions. Reuse `zap_wire` through a local path dependency. Match the egui/eframe versions and dependency conventions used by zapgui rather than independently selecting potentially incompatible versions.

Update the repository's `local_cargo.sh` workspace list so that the new package participates in the existing check, build, and test workflow. Do not introduce a root Cargo workspace unless inspection establishes that the repository has since changed.

Keep the scope limited to the new crate and the existing integration points required by this feature. Do not modify the web client, Android app, chain grammar, or edit-application implementation.

### 2. Add origin metadata to the shared wire protocol

Add a plain enum to `zap_wire`, with exactly these logical origins:

- `Human`
- `Orchestrator`

Add the origin to `Frame`. A client must select its origin explicitly at the boundary: zapcli always sends `Human`; zap_orchestrator always sends `Orchestrator`. Do not infer origin from the request text or the frame kind.

Treat this field as the origin of the logical request/session, not the physical process that most recently wrote a frame. When zapgui sends a response, it echoes the request's origin. Consequently, every response and every stream frame must retain the correct origin throughout the complete request lifecycle.

Inspect the existing serialization before choosing how to encode the field. The current wire header is [kind: u8][length: u32 little-endian][payload]. Do not simply insert an extra header byte without considering an already-running zapgui binary: the old process could misinterpret the new header and leave the connection unusable.

Prefer a backwards-compatible encoding if the existing kind values permit it. One viable design is to reserve an unused bit in the kind byte as the origin discriminator, leaving the lower bits for `FrameKind`. Existing Human frames then retain their current wire representation, while new Orchestrator frames are distinguishable. Verify the enum's actual range before adopting this approach. If it is unsuitable, use an explicitly versioned protocol with safe rejection of incompatible peers. Never silently retry an Orchestrator request as Human.

Keep the existing constructors and call sites compatible where practical, but make origin propagation explicit at the client/server boundaries. Centralize the encoding and decoding logic in `zap_wire`; do not independently implement framing in zapcli, zapgui, and zap_orchestrator.

The origin field is routing metadata, not authentication. Do not rely on it as a security boundary.

### 3. Define the machine response protocol

Human-facing behavior must remain unchanged. Human-originated requests receive the same prose and stream content they receive today. Orchestrator-originated requests receive structured machine-readable data instead of prose that the orchestrator has to interpret heuristically.

Use JSON, serialized and parsed with a proper JSON library. Do not manually concatenate JSON strings.

Every machine-facing response payload must be a valid JSON object with a stable schema version and an explicit event discriminator. Keep the schema small, deterministic, and easy for an LLM to consume.

For example, a final chain result should have the following conceptual shape:

- schema_version: integer
- event: chain_result
- status: a defined outcome enum
- summary: concise description
- steps: per-step results, including an unambiguous index, path, status, and optional error
- run: execution status and exit code, where applicable

Define stable outcomes that distinguish successful application, partial application, denial, cancellation, replacement, validation failure, write failure, and execution failure. Do not collapse these into a single success/failure boolean. In particular, distinguish a denied run step from a failed run, and distinguish a rejected chain from a chain that was partially applied.

Also define the machine payloads for stream events. Preserve the existing `FrameKind` roles, but for Orchestrator-originated output use JSON envelopes with explicit event names, such as stream_open, stream_data, stream_clear, stream_close, and stream_done. Carry arbitrary stream contents inside a JSON string field so that newlines, quotes, and other characters cannot make the protocol ambiguous.

Choose one consistent definition of terminal completion. Every possible chain outcome must result in an unambiguous final machine result, including all error and denial paths. Do not make the orchestrator infer success from connection closure or the absence of an error message.

Keep the response types in a small, shared location if doing so prevents the producer and consumer from drifting apart. Do not change chain parsing or the semantics of applying edits merely to accommodate this response protocol.

### 4. Integrate with the actual zapgui routing behavior

Read the existing listener and router and trace the complete path from incoming frame to review outcome and final socket response.

The supplied source indicates an important distinction: zapcli sends chains containing a run step through `ChainRun`; edit-only chains use a normal `Request` carrying the `chain <cwd>\n<block>` request shape. The existing `ChainRun` handler rejects chains without a run step. Verify this behavior in the current code and preserve the appropriate request type for each chain. Do not send every chain as `ChainRun`.

For Orchestrator-originated requests, add origin-aware routing through the existing chain workflow. Both edit-only and run-containing chains must still use zapgui's established parsing, diff, review, writing, and execution paths.

An Orchestrator-originated chain must never bypass human review. Inspect and disable the existing origin-inappropriate shortcuts, including automatic application of whitelisted chains and the trusted-run fast path. These shortcuts may remain exactly as they are for Human-originated requests. Do not reimplement the writer or chain parser.

This is essential because opening zapgui is not itself approval to apply edits. The popup click authorizes handoff to zapgui; the human must still review and confirm or deny the actual changes in zapgui's normal review window.

The existing run-chain handler has a fixed 600-second review deadline. Inspect every relevant server-side deadline, not just the zapcli client's `REPLY_TIMEOUT`. Preserve existing Human behavior, but do not give Orchestrator-originated review sessions an arbitrary ten-minute expiry when the requirement permits the human to take hours. For the orchestrator path, keep the connection open until a terminal outcome, cancellation, or genuine connection failure. Handle disconnection and cancellation cleanly.

Propagate the request origin into all branches of the routing and outcome code. Do not accidentally return prose, use Human-origin defaults, or omit the final machine result on validation errors, denials, cancellations, or run failures.

### 5. Build the long-lived socket client around an independent reader

The orchestrator connects directly to the zapgui Unix socket using `zap_wire`. It must never execute zapcli or scrape its output. Use the socket location and framing conventions established by the existing code.

Do not reuse the synchronous `zapcli::send()` helper as the orchestrator's session abstraction. Its ten-second deadline exists for ordinary requests. Inspect and reuse the existing `Session` and `open_chain_run()` implementation patterns, which already support background frame reading, incremental stream delivery, and cancellation.

A suitable design is a dedicated socket-session actor:

- It opens the socket and sends the correct origin-tagged frame.
- A dedicated reader thread, or an equivalent asynchronous reader, drains frames incrementally and publishes them through a channel.
- All writes are serialized through one writer path so that control frames cannot interleave with one another.
- The session remains alive throughout review and any subsequent run.
- Cancellation, disconnects, malformed frames, and terminal outcomes become explicit events for the orchestrator.

If the orchestrator uses Tokio for Gemini calls and its event loop, bridge the existing blocking Unix socket APIs through a dedicated reader thread or another appropriate blocking-task boundary. Do not call blocking socket reads, long sleeps, or synchronous waits from the primary async event loop.

The Gemini/event loop must remain responsive while a chain awaits review. Keep review sessions independent from Gemini request processing. Maintain conversation history per workflow; do not concurrently mutate one conversation's chronological history or send a dependent follow-up before the preceding chain's outcome is known. Independent workflows can continue making Gemini calls while another workflow waits.

Respect the rate limiter and retry behavior specified in `gemini_orchestrator_plan.md`. Use asynchronous waiting instead of blocking the event loop.

The current zapgui implementation has a shared pending-chain slot, and an incoming chain can replace an unconfirmed pending chain. Therefore, do not submit several proposed chains to zapgui concurrently. Introduce a single review coordinator that queues proposals and permits at most one active popup/handoff/review session at a time. Continue independent Gemini work in the meantime, but do not send the next proposal into zapgui until the active chain has reached a terminal result. Ensure a replacement can never silently lose an earlier proposal.

### 6. Add the standalone popup binary

Implement the popup as a separate binary inside `zap_orchestrator`, not as a modal widget inside zapgui and not by changing zapgui's existing review UI.

Use the egui/eframe setup already present in the project. Make a small native window positioned at the top-left of the screen, always on top where the platform supports it, and persistent until the user clicks an explicit action. Do not rely on the existing reusable popup widget; it is an in-window modal and is not the required component.

The popup must own the proposed chain payload while waiting for the click. Before the click, it must not connect to zapgui, send a frame, or expose the payload to zapgui.

Use a simple, private IPC mechanism between the orchestrator and popup. Prefer a pipe or stdin/stdout protocol over passing a potentially large chain as command-line arguments. This avoids OS argument-size limits and avoids leaking the full payload through process listings.

A sensible lifecycle is:

1. The orchestrator starts the popup process and gives it the payload through its private IPC channel.
2. The popup stores the payload in its own application state and waits for the human action.
3. On the explicit review click, the popup returns the handoff signal and payload to the orchestrator.
4. Only then does the orchestrator open the zapgui socket session and submit the correctly typed, Orchestrator-originated frame.
5. The popup closes after a successful handoff. If handoff fails, do not silently lose the chain; report the failure and provide a safe way to retry or cancel.

The popup's job is to gate handoff, not to implement chain parsing, edit application, or the full zapgui review interface. Zapgui must not learn about the payload until the click.

### 7. Make the safety restriction a real constraint

Create `zap_orchestrator/IMPORTANT_TODO.md`. It must clearly say that autonomous control of the zap interface is blocked until zap's safety has been verified.

Include an explicit question:

What exactly constitutes verification of zap's safety?

Leave the answer blank. Do not invent verification criteria, infer approval from existing whitelist behavior, or claim the system is verified because these changes compile.

Keep the implementation human-gated: each proposal must require the popup click and then go through zapgui's normal review. No unattended approval, Human-origin impersonation, or shortcut around review may be introduced.

The unresolved definition must remain a blocker for any future change that removes these human approval gates. This TODO is not permission to implement autonomous operation while verification remains undefined.

### 8. Isolate the decisions that have not been made

Three product decisions remain unresolved. Do not silently choose defaults for them:

1. Run trigger: The orchestration plan mentions a watched folder and a push, but neither has been selected. Do not arbitrarily implement one.
2. Gemini API key source: Do not silently decide to use an environment variable, config file, credential store, or another source.
3. Path policy: It has not been decided whether Orchestrator-originated chains may target paths outside the existing whitelist or must remain inside it. Do not choose this policy by assumption.

Implement independent integration components now. Isolate these decisions behind small interfaces or explicit required configuration so they do not infect the wire protocol, popup, or socket-session design. Where a decision is required to safely enable a behavior, fail closed with a clear diagnostic rather than silently selecting a policy.

Do not fabricate a default trigger or credential provider merely to make the binary appear runnable. At the end, ask for the outstanding decisions that genuinely prevent completing the runnable workflow. Clearly distinguish these product decisions from implementation issues you were able to resolve from the source code.

### 9. Tests and verification

Add focused automated tests for:

- Frame origin round-tripping and wire compatibility, including unchanged Human-frame encoding if the existing framing permits it.
- Machine-response JSON serialization, parsing, and stable status values.
- Correct routing of edit-only and run-containing chains.
- Orchestrator-originated requests never taking the auto-apply or trusted-run bypasses.
- Preservation of existing Human behavior and prose.
- Incremental stream delivery and a session that remains pending beyond the ordinary ten-second request deadline.
- Complete final outcomes for success, partial application, denial, cancellation, rejection, execution failure, and connection failure.
- Review coordination that prevents simultaneous submissions from replacing pending chains.
- Popup isolation: no zapgui connection or payload handoff before the click, and exactly one submission after a successful click.
- The existing parser and writer remaining unchanged in semantics.

Run formatting, checks, and tests using the repository's actual project structure. Test the new package and every affected existing package. Report any environment-dependent failures separately from real code failures.

Do not modify chain parsing, the file-editing implementation, or unrelated clients to make the tests pass.

### Deliverable

Implement the new crate and the necessary integration changes. Before finishing, provide a concise report of the architecture, relevant files changed, tests run and their results, any discrepancy discovered between the design description and the actual code, and the unresolved product decisions.

Do not claim a feature works unless it has been implemented and tested, and do not claim the safety gate has been satisfied.
