# Chat UI design QA

Date: 2026-10-12
Viewport: 1507 × 936 CSS pixels, Codex in-app browser, desktop density

Screenshot paths below refer to local acceptance evidence and are not included in Git.

## Sources

- Codex shell and composer: `/var/folders/x_/m176tcjj1l1gxvsdqwl93ly5n_h4wv/T/codex-clipboard-8297f62f-3597-4f4b-bf83-32fc48524985.png`
- Codex permission state: `/var/folders/x_/m176tcjj1l1gxvsdqwl93ly5n_h4wv/T/codex-clipboard-80799c3c-c86b-4ab3-8fd4-8dfd831dbebd.png`
- Codex reasoning slider: `/var/folders/x_/m176tcjj1l1gxvsdqwl93ly5n_h4wv/T/codex-clipboard-dcc00969-9aca-486b-903c-ca045dcfea35.png`
- Codex model list: `/var/folders/x_/m176tcjj1l1gxvsdqwl93ly5n_h4wv/T/codex-clipboard-766fd019-6340-4c88-9071-8f12bb3cae28.png`
- Implementation Space: `prototype/design-qa-space.jpg`
- Implementation model list: `prototype/design-qa-models.jpg`
- Implementation permission menu: `prototype/design-qa-permission.jpg`

## Full-state comparison

The shell uses a restrained Space-first layout: a compact Space project sidebar, a full-size Main Page surface, and a bottom Main Thread dock. The dock is collapsed by default with a pending/latest-message preview above the input and expands into the full conversation over the page. Synthetic window chrome, the narrow rail, recent-chat duplication, canvas action bar and repeated project-directory row are omitted. View Thread selection swaps the single page surface to the selected View page; no node canvas competes with the generated page.

Focused states were checked at the same viewport:

- Model, effort and speed share one anchored control. Every trigger opens the same compact popover; the model and speed lists return to the same surface, while the effort track supports pointer drag, labels and arrow keys.
- The control and permission menus use the compact Codex scale: title emphasis is restrained, list rows stay readable, and hover/press/selection states use short easing transitions.
- Model list contains the live seven-model catalog from the installed desktop Codex and uses the reference hierarchy (`选择模型`, `默认`, `推荐模型集`, flat rows, current check).
- Reasoning picker keeps the reference compact card, blue effort track, thumb and capability-dependent options. The default `GPT-6.1 Sol Max` state matches the supplied reference when a new Main Thread is opened.
- Permission picker keeps the three reference modes, descriptions, current check and `了解更多` link. The Rust service sends the corresponding approval reviewer and sandbox policy for every thread turn.
- Space shows a Main Page and a tree of runnable Views. View documents render in the same isolated `iframe` surface when their Thread is selected.

## Interaction checks

- The Space sidebar switches projects and exposes Main Thread plus the primary board.
- Directory dialog browses, chooses and persists a project.
- View selection and page switching are wired to the Rust service and `.space/space.json`; orchestration metadata remains persisted even though the UI no longer exposes a node canvas.
- Main Thread conversation opens from a View or the composer, renders streamed markdown, shows activity/errors and handles approval replies.
- Main Thread stays in a bottom dock: the collapsed state shows the pending/latest message and the expanded state shows the full feed. Composer supports file attachment into the prompt, model/effort/speed selection, permission selection, send and interrupt. Unreliable speech input, the redundant computer context chip and the repeated project-directory row are intentionally omitted.
- The unified control closes from Escape or an outside click, and each trigger shares the same panel state rather than creating separate menus.

## Findings and fixes

- Replaced the former single-file chat surface with domain-backed Space/View state and dedicated page, conversation, composer, picker and directory components.
- Added typed permission validation, correct `dangerFullAccess` object serialization, project-scoped policy roots, bundled desktop Codex discovery and View Thread creation/routing.
- Added OpenAI Sans and source Codex icon assets; removed the old oversized picker and dead controls.
- Split the Rust runtime into process, project, Space and chat service modules while keeping one shared state boundary.
- Removed dead canvas undo/redo state and the unused voice/context controls.

## Validation

- `pnpm test`: passed (8 tests)
- `pnpm build`: passed; Sites artifacts generated
- `cargo test --lib`: passed (6 tests)
- Browser verification: passed in the Codex in-app browser at `http://127.0.0.1:1420/`; checked Main Page rendering, View Thread page switching, collapsed and expanded bottom dock states, and model popover anchoring.

final result: passed
