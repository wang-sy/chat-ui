# Chat UI implementation plan

The product contract is `SPEC.md`. The Codex screenshots define the shell and composer, not a replacement for the product's Space and View model.

## User experience

Each project opens a persistent Space. A single Main Thread coordinates the Space through a bottom composer dock. Space is the default page surface; the dock stays collapsed around pending work and expands for explanations, approvals and questions without replacing the generated page. A View is a runnable isolated web page, with its own thread, state, version and provenance. The project page opens Main Page, and selecting a View Thread swaps the same surface to that View. The same composer can target a selected View and clearly identifies its recipient.

The empty state, model list, effort slider and permission chooser follow the supplied Codex desktop references. Model capabilities come from the installed Codex runtime. The app never presents an unavailable model or an unsupported effort as usable.

## Boundaries

| Layer | Owns | Does not own |
| --- | --- | --- |
| React components | Presentation, input, accessible controls, isolated iframe lifecycle | Filesystem writes, Codex protocol |
| Workspace controller | Selection, polling, pending actions, errors | Page internals |
| Local HTTP service | Authentication, validated commands, shared snapshots | Model inference |
| Space domain | View registry, page versions, relationships and events | Child DOM access |
| Orchestrator | Main/View ownership, dynamic tools, task routing | Unrestricted cross-View writes |
| Codex transport | One external app-server, request correlation, streaming, approvals | Product layout |
| Storage | Atomic project-scoped `.space` persistence | Rewriting unrelated projects |

## Runtime contract

- One persistent Main Thread per project. Existing chats are preserved and can be opened; a migrated project adopts its most recent conversation as its Main Thread.
- The Main Thread is created with explicit Chat UI instructions and a structured output contract. Its `space_plan` is a complete composition snapshot: it updates the Main Page manifest, creates or reuses View records, replaces connections, and uses explicit `removeViews` IDs for deletion. The runtime starts one View Thread per planned page. Each View Thread receives its own brief and a structured page result; the runtime only publishes that result back to the owning View.
- Both roles include the same `prompts/visual.md` decision policy at thread start/resume. Main states the representation and reason in each brief; View checks it against the actual goal. Integration receives the same policy. HTML remains the delivery container; explanation tasks default to meaningful diagrams/charts, while real forms/editors and concise direct answers remain supported. This is a model behavior contract, not a brittle SVG-keyword validator.
- View pages use an iframe with scripts enabled and without same-origin authority. A scoped, source-checked postMessage bridge reports ready/state/decision/focus events. View pages never receive the local service token. State/event payloads are bounded.
- Main embeds live child artifacts through `chat-view` slots. `pageRuntime` owns slot sizing, isolated child frames, catalog updates, navigation and local linked selection. Main owns the surrounding visual relationships. Updating one artifact leaves sibling frames mounted; diagram and chart tasks prioritize SVG rather than introductory web pages.
- The composition includes every retained View as a bare iframe: no host title bar, status badge, border or controls. Main owns presentation and optional accessible folding; the host preserves authored layout and mounted frames. `childSize` reports source-checked dimensions for default automatic height, overridable with CSS or `--chat-view-height`. Missing Views are appended without restructuring Main's content. DOM behavior tests cover coverage, authored folding, local state, sizing, updates and message ownership.
- Each assignment can schedule one integration turn after its assigned Views are ready. Persisted composition state prevents repeat execution; the integration schema and domain validation permit only Main Page and relationship updates, never child reassignment or deletion. Child summaries provide the factual integration context. Failure preserves the working artifacts.
- Documents are stored beneath `.space/views/<id>/`. Each accepted page result advances the persisted View version and replaces the current HTML; historical version browsing and restore are intentionally outside this prototype. Page selection and status are persisted as Space state.
- One service is shared by browser and Tauri. No second model runtime or simulated response path is used in production.
- Permissions are typed and validated at every turn: request approval, automatic risk review, or full access. Selection is preserved per thread and is not silently escalated.

## Acceptance

1. A fresh project opens a Space with the Codex-style composer and actual runtime models.
2. A user request can create an owned View Thread, publish interactive HTML and update its visible status.
3. Selecting View Threads and restoring their page/version state survives reload with bounded, persisted Space state.
4. Actions from a View reach its owner; a child cannot modify another View or the parent composition.
5. Conversations, approvals, streamed output, errors, interruption and retry remain usable.
6. The model list, effort slider and permission menu are compared against the supplied references at matching scale.
7. Rust domain/protocol tests, browser interactions, the production build and a local service smoke check pass before handoff. A full model-generation run remains environment-dependent.
