# State consistency and indexing

Updated 2026-11-04 after implementation and live UI validation.

## Implemented behavior

### Connection recovery

The app reports an account connected only after the notification subscription succeeds. Socket/subscription failures produce disconnected events and retry with exponential backoff, capped at 30 seconds. A stable connection resets the backoff. Closing the backend cancels notification retries and history synchronization.

Every successful subscription requests server reconciliation. The selected conversation and open thread refresh after bootstrap and after their history catch-up finishes. RPC calls have time limits, including callback-based profile requests, so an unresponsive daemon cannot leave them waiting indefinitely.

`SyncService` owns a queue per backend. It coalesces conversation requests from bootstrap, inbox paging, and reconnects, retaining requests that arrive during an existing job. Failed conversations retry from their last saved position.

### Startup reconciliation

`BootstrapLoaded` identifies cache and server payloads. Cache initializes each account once; late cache payloads are ignored by the reducer. Subsequent server payloads merge conversation metadata and bindings without clearing reaction/pin/team state, navigation, drafts, pending sends, or the open thread. Only the first hydration may choose a default conversation. Refresh effects target the user's current selection.

RocksDB bootstrap persistence also merges conversation pages instead of deleting the rest of the cached inbox. Selected messages are published from their canonical cache records after persistence, respecting edits and deletions.

### Unread state and conversation identity

Keybase badge snapshots are authoritative, including conversations absent from a sparse full snapshot. Badge subscription and startup/reconnect reconciliation update both the reducer and cache; stale inbox summaries cannot restore old counts. Confirmed local reads are partial updates. An active conversation opened while unread remains in the Unread section until navigation away, even after its authoritative counts reach zero; this display-only retention never inflates badges.

Team metadata distinguishes simple teams from complex teams. Simple teams join DMs in the recent Conversations section while retaining channel identity and permissions. Complex teams remain grouped. The recent list uses monotonic server activity with conversation ID as a stable tie breaker; live notifications refresh existing conversations too. Cached startup no longer fabricates activity timestamps. A backward-compatible cache marker treats legacy synthetic timestamps as unknown until real inbox metadata replaces them, and cache ordering derives from canonical summaries rather than stale secondary index keys. Conversation merges reconcile IDs across both buckets and preserve known team classifications when older payloads omit them.

Internal Keybase topics, including EMOJI and EMOJICROSS, are excluded from normal conversation ingestion. A metadata-only cache repair records older entries' topic types, hides internal entries without deleting their messages, and prevents late cache snapshots from restoring them. This fixes hidden emoji-support conversations appearing as duplicate DMs.

### Profile refresh and social lists

Cached avatars are published even for accounts whose display name is their username. Missing refresh fields preserve known cached images, stale results cannot replace newer images, and a changed avatar URL invalidates the old local image. Profile summaries and the open profile share avatar enrichment.

Social-list loading and failure states are keyed by user and list type. A Followers failure cannot clear Following's loading state or masquerade as an empty result; the profile displays a retry action. A real Keybase daemon restart cleared a latched LevelDB disk-full error, after which the Followers endpoint returned the expected 198 entries.

### Production model isolation and theme consistency

Production view models start empty and project canonical store data. Navigation clears transient state and never fills loading gaps with sample messages, typing indicators, search results, or participants. Fixture constructors and synthetic timeline inflation compile only for tests or the explicit `demo-fixtures` feature; runtime demo mode also requires backend isolation and a dedicated data directory.

Dark mode uses Keybase's neutral surfaces and semantic colors. Selectable text captures link and selection colors when configured, so deferred rendering keeps the parent theme. The emoji picker uses an opaque panel background.

### Message and attachment consistency

Message writes and deletion checks are serialized in the local store. Edit revision comparison uses Keybase's edit message ID and edit timestamp. Older snapshots cannot overwrite newer edits. Edits preserve the original message timestamp and thread identity.

Deletion markers persist across restarts and prevent old message pages or download completions from restoring deleted messages. The reducer also tracks deletions so delayed in-memory pages cannot restore them.

One canonical merge owns attachment path preservation in the cache, timeline, and thread. Downloads patch only local attachment paths onto the latest matching attachment by name, kind, and byte size. A nonempty incoming local path wins over older cached paths; replacement and reordered attachments never inherit paths by position. Full source paths and previews round-trip through the cache; the appended source field remains compatible with older records. They do not replace text, reactions, thread fields, or whole message records. Completion for a deleted message is ignored. The UI receives an attachment-only event.

The original historical-download bug remains fixed: the common ingestion function does not schedule media hydration. Actual live notifications and explicit conversation loads retain their existing media behavior.

### Operation failures

The router handles every effect independently, keeping successful events and running later commands after a failure. Routing/backend errors become command-specific failure events. Primary asynchronous bootstrap, profile, social-list, and conversation loads also settle on early failure.

Failed searches settle their loading state; failed conversation creation retains the dialog and shows an error; failed sends retain a failed pending message. Failed history paging preserves its cursor for retry. Reaction operations carry operation IDs for confirmation/rollback; an old failure cannot roll back a newer operation. Deletes remain visible until the backend accepts them. Edit/delete RPC work runs off the UI thread.

Store error notifications are drained into the visible toast model rather than accumulating invisibly in reducer state.

### Search and missed-message recovery

RocksDB is the source for search. A background observer replays a consistent local database snapshot, then consumes a bounded feed of message-ID invalidations, resolving each ID from the current cache. Snapshot creation and subscription registration are coordinated with message writes, so writes during replay are not lost. Overflow invalidates the subscription and triggers a fresh snapshot; cache writes never wait for indexing and unchanged canonical upserts emit no invalidation. Both the invalidation and index-writer queues are bounded; replay does not read attachment files or call Keybase.

Replaying the cache repairs schema rebuilds, interrupted commits, and stale deleted index entries. Message edits and deletions update the index directly from committed cache changes. Continuous ingestion no longer postpones commits until the queue becomes idle. Cache schema resets notify the observer too.

Network catch-up uses a separate durable position per conversation, independent of Tantivy and historical crawl checkpoints. It reads raw message operations after that position with attachment hydration and automatic read marking disabled, applies messages/edits/reactions/deletions in order, and advances only after the page is saved. Offline responses and failures do not advance the position. Progress uses transport operation IDs, since an edit's visible message ID refers to its original message. Hidden placeholders and expired ephemeral envelopes advance progress without creating visible messages. Transient unboxing failures, unresolved placeholders, and malformed/non-progressing pages retry without advancing the checkpoint or reporting completed sync.

For existing installations, the new position initially starts at zero. Establishing it requires a one-time pass over message metadata, including conversations that the old crawler marked complete. Later catch-up reads only operations after the saved position. This also recovers messages received while zBase was closed. Search rebuilds need only the local database.

Canonical cache writes use direct key lookups rather than scanning a conversation on every new message, and root messages avoid unnecessary ancestor lookups.

### Compact windows and message context

New windows start at a compact 780-point width and a taller 880-point height; the minimum remains 780 × 680. Opening a thread/profile pane expands a narrow window and remembers its previous width; closing it restores that width. Already-wide windows use existing space, manual resizing is respected, and switching pane content does not repeatedly expand the window. Native size changes are submitted after the current frame. While expansion is pending, the chat retains its existing layout and the pane waits for the content-size update; while shrinking, the chat pins both its width and flex basis to avoid a minimum-width text-layout pass, and a snapshot keeps the previous pane contents visible until the smaller native size arrives. Repeated activation of the same profile or thread closes it; a different target replaces its content. Files, Search, Details, and Members use the same toggle/close path. A requested pane uses an overlay when screen/fullscreen constraints prevent enough expansion.

The reaction strip shares one placement policy across timeline messages, thread replies, and right-click activation. Its measured footprint includes recent-reaction and delete buttons. Placement tries the right, above, then below with a pointer gap; it never clamps a right-hand candidate underneath the pointer. Hidden strips are absent from the element tree and cannot intercept clicks.

Thread replies retain their message text and link interactions in the main chat. A cached parent-author/text preview gives context without extra history or file reads, and thread actions show reply counts. Group DMs use the same participant-avatar projection and overlapping-avatar component in the sidebar and conversation header; missing photos use individual initials rather than a channel hash icon.

## Validation

The latest full run passed all 240 tests: 238 app tests and 2 protocol inventory tests. One opt-in live test is ignored by default. The suite runs with:

```sh
cargo test --all-targets --features gpui/runtime_shaders
```

GPUI's runtime-shader feature avoids the missing Xcode offline Metal compiler on this machine without changing the production dependency configuration.

Tests cover reconnect/resubscription against a mock Unix-socket daemon, rejected subscriptions never reporting online, startup refresh preserving navigation/drafts/thread/pending messages, stale cache responses, edits and deletions during attachment downloads, deletion markers surviving restart, snapshot/subscription ordering, search replay/edit/delete behavior, sync queue coalescing/cancellation, independent catch-up positions, failed command isolation, retained paging cursors, and reaction failure ordering. Existing reducer, rendering, RPC inventory, cache, and search tests run as part of the suite.

## Verification limits and follow-up work

- Live read-only benchmarks and native UI checks used disposable copies of the local cache. They verified navigation, Quick Switcher, search, fixture isolation, theme rendering, and hidden-topic cache repair. A real daemon restart was exercised while recovering a latched database error. A debug layout capture across native pane collapse verified that visible timeline text keeps a constant width through all captured frames. Interrupted catch-up, long memory soaks, and large-history disk-usage tests remain unverified; mock daemon tests also cover transport recovery.
- Existing downloaded Keybase files are not removed. Live attachments and loaded conversation pages still automatically download media, including full videos and generic files. An explicit-download UX and cache budget remain separate work.
- Search replays local message metadata in the background on every launch for crash recovery. It can show partial results while rebuilding; a durable index journal could reduce startup indexing work for very large text histories.
- Attachment matching uses the identity available in the current domain model: name, kind, and byte size. Carrying the backend asset ID would distinguish replacement files with identical metadata.
- This remains the app's existing single-Keybase-account flow. A future account-switching implementation needs account-scoped cache partitions and session generations throughout all worker results.
